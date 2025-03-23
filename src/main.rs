use std::ffi::OsString;

mod cli;
mod parser;

use clap::Parser;
use cli::Main;
use itertools::Itertools;
use sqlx::SqlitePool;
use tokio_stream::StreamExt;

use tracing::{self, info_span};
use tracing_indicatif::{span_ext::IndicatifSpanExt, IndicatifLayer};
use tracing_subscriber::Layer;
use tracing_subscriber::{self, layer::SubscriberExt, util::SubscriberInitExt};

// TODO
// - [ ] Since TFIDF is a per document value, how to aggregate tfidf for all documents?
//          - maybe can use stdev? because uncommon words tend to vary in terms of usage

#[derive(Debug, PartialEq)]
struct Item {
    document: Option<String>,
}

#[tokio::main]
async fn main() {
    let command = Main::parse();
    match command {
        Main::Stream(streamer) => stream(streamer).await,
        Main::Rank(ranker) => rank(ranker).await,
    }
}

#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct TermTable {
    document: String,
    term: String,
    lower: String,
    occurence: i64,
    frequency: Option<f64>,
}

fn rank_term_frequency(items: &Vec<TermTable>) -> Vec<f64> {
    items
        .iter()
        .map(|i| i.frequency.unwrap_or(0.))
        .collect_vec()
}

fn rank_inv_document_freuqency(items: &Vec<TermTable>, total_docs: u64) -> f64 {
    (total_docs as f64 / (1. + items.len() as f64)).log10()
}

fn rank_tf_idf(items: &Vec<TermTable>, total_docs: u64) -> Vec<f64> {
    let idf = rank_inv_document_freuqency(items, total_docs);
    rank_term_frequency(items)
        .iter()
        .map(|i| i * idf)
        .collect_vec()
}

async fn rank(arg: cli::Rank) {
    let pool = SqlitePool::connect(env!("DATABASE_URL"));
    let pool = pool.await.unwrap();

    let word_lower = arg.word.to_lowercase();

    let item = sqlx::query_as! {TermTable, "SELECT * FROM term_info WHERE lower = ?", word_lower}
        .fetch_all(&pool)
        .await
        .unwrap();

    let total_docs = sqlx::query! {"SELECT COUNT(document) as count FROM doc_info"}
        .fetch_one(&pool)
        .await
        .unwrap();

    println!(
        "IDF for {} is {:.5}",
        word_lower,
        rank_inv_document_freuqency(&item, total_docs.count as u64)
    );
    let ranks = rank_tf_idf(&item, total_docs.count as u64);
    for (tfidf, term) in ranks.iter().zip(item).take(10) {
        println!("TFIDF is {:.5} for {:?}", tfidf, term);
    }
}

async fn stream(arg: cli::Stream) {
    let indicatif_layer = IndicatifLayer::new();
    let _subscriber = tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_filter(tracing_subscriber::filter::filter_fn(|metadata| {
                    metadata.target() == "carigali"
                })),
        )
        .with(indicatif_layer)
        .init();

    let pool = SqlitePool::connect(env!("DATABASE_URL"));
    let pool = pool.await.unwrap();

    let processed =
        sqlx::query_as! {Item,"SELECT DISTINCT(document) FROM doc_info"}.fetch_all(&pool);
    tracing::info! {DATABASE_URL=env!("DATABASE_URL"), "Establishing connection:"};

    let pbar_parse_span = info_span!("parser");
    let pbar_skips_span = info_span!("skips");

    pbar_parse_span.pb_set_style(
        &indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len}| {per_sec:7} | {msg} {eta:3}",
        )
        .unwrap()
        .progress_chars("##-"),
    );
    let processed = processed
        .await
        .unwrap_or(vec![])
        .into_iter()
        .map(|i| OsString::from(i.document.unwrap_or_default()))
        .collect::<Vec<OsString>>();
    let skip_len = processed.len() as u64;
    pbar_skips_span.pb_set_style(
        &indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len} | {eta:3}",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    pbar_parse_span.pb_set_length(arg.size);
    pbar_skips_span.pb_set_length(skip_len);

    let pbar_bytes = indicatif::ProgressBar::hidden();

    let mut count: u64 = 0;
    tracing::info! {"--root"=arg.root.to_str(), "Reading `--root`:"};
    for i in arg.root.read_dir().unwrap() {
        if count == arg.size {
            break;
        }
        let name = i.unwrap();
        if processed.contains(&name.file_name()) {
            pbar_skips_span.in_scope(|| {
                tracing::info! {target: "carigali", filename=&name.file_name().to_str().unwrap(), "Skipping"}
            });
            pbar_skips_span.pb_inc(1);
            continue;
        }
        let n = name.file_name().into_string().unwrap();
        pbar_parse_span.in_scope(|| {
            tracing::info! {target: "carigali", filename=n, "Reading file."};
        });
        let q1 = sqlx::query! {
            r#"INSERT INTO doc_info (document, term_count) VALUES (?, 0)"#,
            n
        }
        .execute(&pool);

        let tokens = crate::parser::tokenize_file(name.path());
        let ngram2_tokens = crate::parser::ngram2(&tokens);
        let ngram3_tokens = crate::parser::ngram3(&tokens);

        let _ = q1.await.unwrap();

        let mut stream_tok = tokio_stream::iter(&tokens);
        while let Some(t) = stream_tok.next().await {
            if t.is_text() {
                t.register(&n, &pool).await.unwrap();
            };
        }
        let mut stream_ng2 = tokio_stream::iter(&ngram2_tokens);
        while let Some(ng2) = stream_ng2.next().await {
            ng2.register(&n, &pool).await.unwrap();
        }
        let mut stream_ng3 = tokio_stream::iter(&ngram3_tokens);
        while let Some(ng3) = stream_ng3.next().await {
            ng3.register(&n, &pool).await.unwrap();
        }

        let n_bytes = n.as_bytes().iter().fold(0, |acc, curr| acc + *curr as u64);
        pbar_bytes.inc(n_bytes);
        pbar_parse_span.in_scope(|| {
            tracing::info! {target: "carigali", filename=n, "Done reading file at {:.3} B/s", pbar_bytes.per_sec()};
        });
        pbar_parse_span.pb_inc(1);
        count += 1
    }
}

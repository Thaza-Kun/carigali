use std::ffi::OsString;

mod cli;
mod parser;

use clap::Parser;
use indicatif::MultiProgress;
use sqlx::SqlitePool;
use tokio_stream::StreamExt;

// TODO
// - [ ] Since TFIDF is a per document value, how to aggregate tfidf for all documents?
//          - maybe can use stdev? because uncommon words tend to vary in terms of usage

#[derive(Debug, PartialEq)]
struct Item {
    document: Option<String>,
}

#[tokio::main]
async fn main() {
    let pool = SqlitePool::connect(env!("DATABASE_URL"));
    let arg = cli::Args::parse();

    let pool = pool.await.unwrap();
    let processed =
        sqlx::query_as! {Item,"SELECT DISTINCT(document) FROM doc_info"}.fetch_all(&pool);
    let pbar_parse = indicatif::ProgressBar::new(arg.size).with_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len} | {per_sec:7} | {msg} {eta:3}",
        )
        .unwrap()
        .progress_chars("##-"),
    ).with_finish(indicatif::ProgressFinish::Abandon);

    let processed = processed
        .await
        .unwrap()
        .into_iter()
        .map(|i| OsString::from(i.document.unwrap_or_default()))
        .collect::<Vec<OsString>>();
    let skip_len = processed.len() as u64;
    let pbar_skip = indicatif::ProgressBar::new(skip_len).with_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7}/{len} | {per_sec:7} | {msg} {eta:3}",
        )
        .unwrap()
        .progress_chars("##-"),
    ).with_finish(indicatif::ProgressFinish::Abandon);

    let mut count_iter = 0;

    let pbar_multi = MultiProgress::new();
    let pbar_skip = pbar_multi.add(pbar_skip);
    let pbar_parse = pbar_multi.add(pbar_parse);

    for i in arg.root.read_dir().unwrap() {
        let name = i.unwrap();
        if count_iter == arg.size {
            break;
        }
        if processed.contains(&name.file_name()) {
            // SKIPPING manually because `.take_while()` doesn't seem to work with large directory entries.
            pbar_skip.set_message(format!("{}:SKIP", name.file_name().into_string().unwrap()));
            pbar_skip.inc(1);
            pbar_parse.reset_elapsed();
            continue;
        }
        let n = name.file_name().into_string().unwrap();
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

        pbar_parse.set_message(format!("{}", name.file_name().into_string().unwrap()));
        count_iter += 1;
        pbar_parse.set_position(count_iter);
    }
}

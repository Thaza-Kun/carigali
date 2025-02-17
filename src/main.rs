use std::ffi::OsString;

mod cli;
mod parser;

use clap::Parser;
use sqlx::SqlitePool;

// TODO
// - [ ] Since TFIDF is a per document value, how to aggregate tfidf for all documents?
//          - maybe can use stdev? because uncommon words tend to vary in terms of usage

#[derive(Debug, PartialEq)]
struct Item {
    document: Option<String>,
}

#[tokio::main]
async fn main() {
    let arg = cli::Args::parse();
    let pool = SqlitePool::connect(env!("DATABASE_URL")).await.unwrap();
    let processed = sqlx::query_as! {Item,"SELECT DISTINCT(document) FROM doc_info"}
        .fetch_all(&pool)
        .await
        .unwrap()
        .into_iter()
        .map(|i| OsString::from(i.document.unwrap_or_default()))
        .collect::<Vec<OsString>>();
    println!("{}", &processed.len());
    let pbar = indicatif::ProgressBar::new(arg.size).with_style(
        indicatif::ProgressStyle::with_template(
            "[{elapsed_precise}] {bar:40.cyan/blue} {pos:>7} | {per_sec:7} | {msg} {eta:3}",
        )
        .unwrap()
        .progress_chars("##-"),
    );

    let mut count = 0;
    for i in arg.root.read_dir().unwrap() {
        let name = i.unwrap();
        if count == arg.size {
            break;
        }
        if !processed.contains(&name.file_name()) {
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

            let mut tasks_tok = Vec::new();
            let mut tasks_ng2 = Vec::new();
            let mut tasks_ng3 = Vec::new();

            for t in &tokens {
                if t.is_text() {
                    tasks_tok.push(t.register(&n, &pool));
                };
            }
            for ng2 in &ngram2_tokens {
                tasks_ng2.push(ng2.register(&n, &pool));
            }
            for ng3 in &ngram3_tokens {
                tasks_ng3.push(ng3.register(&n, &pool));
            }
            let _ = tasks_tok
                .into_iter()
                .map(|t| async { t.await.unwrap() })
                .collect::<Vec<_>>();
            let _ = tasks_ng2
                .into_iter()
                .map(|t| async { t.await.unwrap() })
                .collect::<Vec<_>>();
            let _ = tasks_ng3
                .into_iter()
                .map(|t| async { t.await.unwrap() })
                .collect::<Vec<_>>();
            pbar.set_message(format!("{}", name.file_name().into_string().unwrap(),));
            count += 1;
            pbar.set_position(count);
        }
    }
    pbar.finish();
}

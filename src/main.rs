#![allow(unreachable_code)]

use std::{collections::HashMap, ffi::OsString};

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
    let pool = SqlitePool::connect("sqlite://./test/test.db")
        .await
        .unwrap();
    let processed = sqlx::query_as! {Item,"SELECT DISTINCT(document) FROM term_info"}
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
            pbar.set_message(format!("{}", name.file_name().into_string().unwrap(),));
            count += 1;
            pbar.set_position(count);
            let tokens = crate::parser::tokenize_file(name.path());
            let mut dict: HashMap<String, u64> = HashMap::new();
            for t in &tokens {
                if !t.is_text() {
                    continue;
                };
                let t = (t.clone()).into();
                match dict.contains_key(&t) {
                    true => {
                        let c = dict.get(&t).unwrap();
                        dict.insert(t, c + 1);
                    }
                    false => {
                        dict.insert(t, 1);
                    }
                }
            }
            let n = name.file_name().into_string().unwrap();
            for (k, v) in dict {
                let v = v as i32;
                let lower = k.to_lowercase();
                let _ = sqlx::query! {
                r#"INSERT INTO term_info (document, term, lower, occurence) VALUES (?, ?, ?, ?)"#,
                n, k, lower, v
            }
            .execute(&pool)
            .await
            .unwrap();
            }
            for (t1, t2) in crate::parser::ngram2(&tokens) {
                let term = format!("{} {}", &t1, &t2);
                let lower1 = t1.to_lowercase();
                let lower2 = t2.to_lowercase();
                let _ = sqlx::query! {
                r#"INSERT INTO ngram_two (document, term, lower1, lower2) VALUES (?, ?, ?, ?)"#,
                        n, term, lower1, lower2
                    }
                .execute(&pool)
                .await
                .unwrap();
            }
            for (t1, t2, t3) in crate::parser::ngram3(&tokens) {
                let term = format!("{} {} {}", &t1, &t2, &t3);
                let lower1 = t1.to_lowercase();
                let lower2 = t2.to_lowercase();
                let lower3 = t3.to_lowercase();
                let _ = sqlx::query! {
                    r#"INSERT INTO ngram_three (document, term, lower1, lower2, lower3) VALUES (?, ?, ?, ?, ?)"#,
                    n, term, lower1, lower2, lower3
                }
                .execute(&pool)
                .await
                .unwrap();
            }
        } else {
            pbar.set_message(format!(
                "SKIPPING:{}",
                name.file_name().into_string().unwrap(),
            ));
            // For some reason counter is increased even though the true case block is skipped.
            // So we manually set the counter to zero.
            // Although logically unsound, practically, it gives the same effect as counting to the desired size
            // because file numbers are large anyway.
            count = 0;
        }
    }
    pbar.finish();
}

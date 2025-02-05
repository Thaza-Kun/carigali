use std::collections::VecDeque;
use std::fs::create_dir;
use std::path::{self, PathBuf};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::u64;
use subprocess::Popen;

use clap::Parser;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Args {
    #[arg(short)]
    indir: path::PathBuf,
    #[arg(short)]
    outdir: path::PathBuf,
    #[arg(short)]
    size: Option<u64>,
}

#[derive(Deserialize, Serialize)]
struct Data {
    content: String,
}

struct RingBuffer {
    size: usize,
    data: VecDeque<u16>,
}

impl RingBuffer {
    fn new(size: usize) -> RingBuffer {
        RingBuffer {
            size,
            data: VecDeque::with_capacity(size),
        }
    }

    fn push(&mut self, data: u16) {
        if self.data.len() == self.size {
            self.data.pop_front();
        }
        self.data.push_back(data);
    }

    fn average(&mut self) -> f64 {
        (self.data.iter().fold(0, |i, f| i + f) / self.data.len() as u16).into()
    }
}

/// We call JS to parse wikipedia content because we do not want to implement a parser
/// where someone would call the implementation with the name `wtf`
///
/// In JS, there is a poor soul already maintaing `wtf-wikipedia`
#[tokio::main]
async fn main() {
    let args = Args::parse();
    #[cfg(target_os = "windows")]
    let bunpath: PathBuf = PathBuf::from("C:/Users/LENOVO/.bun/bin/bun.exe");
    let mut bun = Command::new(bunpath.clone());
    println!("Checking bun version");
    if let Err(e) = bun.arg("--version").status() {
        panic!("[ERROR {}] Command not found in {}", e, bunpath.display())
    }
    println!("Running bun server");
    let mut bunspawn = Popen::create(
        &[
            bunpath.clone().into_os_string(),
            "run".into(),
            "parse".into(),
        ],
        Default::default(),
    )
    .unwrap();
    bunspawn.detach();
    sleep(Duration::new(1, 0));
    let pbar = match args.size {
            Some(s) => indicatif::ProgressBar::new(s).with_style(
                indicatif::ProgressStyle::with_template("{spinner:.green} <{msg}> [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos}/{human_len} ({per_sec}, {eta})").unwrap().progress_chars("#>-")),
                None => indicatif::ProgressBar::no_length().with_style(indicatif::ProgressStyle::with_template(
        "{spinner:.green} <{msg}> [{elapsed_precise}] [{wide_bar:.cyan/blue}] {human_pos} ({per_sec})",
                )
                .unwrap()
                .progress_chars("#>-")),
                };
    let _ = create_dir(&args.outdir);
    let client = reqwest::Client::new();

    let outdir = args.outdir;
    let mut ringbuf = RingBuffer::new(2000);

    let mut reader = tokio::fs::read_dir(&args.indir).await.unwrap();
    while let Ok(Some(entry)) = reader.next_entry().await {
        let outfilename = entry.file_name().into_string().unwrap();
        let path = entry.path();
        let outfile = path::PathBuf::from_iter([&outdir, &format!("{}.md", outfilename).into()]);
        pbar.inc(1);
        if outfile.as_path().exists() {
            pbar.set_message(format!("{}:SKIP", outfilename));
            continue;
        }
        let content = tokio::task::spawn_blocking(move || tokio::fs::read_to_string(path))
            .await
            .unwrap()
            .await
            .unwrap();
        ringbuf.push(content.as_bytes().iter().fold(0, |a, b| a + (*b as u16)));
        pbar.set_message(format!(
            "{}:{:>10.3} MB/s",
            outfilename,
            ringbuf.average() * pbar.per_sec() / 1024.0
        ));
        match client
            .post("http://localhost:3010/wikitext")
            .json(&Data { content })
            .send()
            .await
        {
            Ok(res) => {
                let contents = res.text().await.unwrap();
                let _ = tokio::task::spawn_blocking(move || {
                    std::fs::write(outfile, contents).unwrap();
                })
                .await
                .unwrap();
            }
            Err(e) => println!("{}", e),
        }
    }
    println!("Killing bun server");
    bunspawn.terminate().unwrap();
}

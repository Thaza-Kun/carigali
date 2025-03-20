use std::fs::{create_dir_all, File};
use std::io::{self, BufRead, Write};
use std::path::{self, Path};

use clap::Parser;

use quick_xml::errors::IllFormedError;
use quick_xml::name::QName;
use quick_xml::Reader;

use serde::{Deserialize, Serialize};
use serde_yaml as yaml;

use indicatif::ProgressIterator;

fn main() {
    let args = Args::parse();
    let mut filepath = args.wikipath.clone();
    filepath.push(args.filename);
    let filesize = filepath.metadata().unwrap().len();
    let pbar = indicatif::ProgressBar::new(filesize);
    pbar.set_style(indicatif::ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({bytes_per_sec}, {eta})")
        .unwrap()
        .progress_chars("#>-"));
    println!("Reading from {}", filepath.display());
    let data = parse_xml(read_lines(filepath).unwrap(), &pbar);
    pbar.finish_with_message("done");
    pbar.reset();
    let mut dir = args.wikipath.clone();
    dir.push(args.output.unwrap_or("raw".into()));
    // path::PathBuf::from("testoutput/raw");
    if let Err(_) = create_dir_all(dir.clone()) {}
    println!("Output: {}\\{{revision.id}}.wiki", dir.display());
    for d in data.iter().into_iter().progress_with_style(pbar.style()) {
        let rev = d.revision.first().unwrap();
        let content = format!(
            "---\n{}---\n{}",
            yaml::to_string(&d).unwrap_or_default(),
            rev.text.clone().unwrap_or_default()
        );
        let file = path::PathBuf::from_iter(vec![&dir, &format!("{}.wiki", &rev.id).into()]);
        let _f = File::create(file)
            .unwrap()
            .write(content.as_bytes())
            .unwrap();
    }
}

#[derive(clap::Parser)]
struct Args {
    #[arg(long)]
    wikipath: path::PathBuf,
    #[arg(long)]
    filename: String,
    #[arg(long, help = "Relative to --wikipath")]
    output: Option<String>,
}

/// Page
/// ```md
/// ---
/// title: {string}
/// ns: {string}
/// id: {string}
/// revision:
///     id: {string}
///     parentid: {string}
///     timestamp: {string}
///     contributor:
///         username: {string}
///         id: {string}
///     comment: {string}
///     model: {string}
///     format: {string}
///     sha1: {string}
/// ---
/// <text>
/// ```
#[derive(Serialize, Deserialize, Debug)]
struct Page {
    title: String,
    ns: u64,
    id: u64,
    revision: Vec<Revision>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Contributor {
    username: Option<String>,
    id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Revision {
    id: String,
    parentid: Option<String>,
    timestamp: String,
    contributor: Contributor,
    comment: Option<String>,
    model: String,
    format: String,
    #[serde(skip_serializing)]
    text: Option<String>,
    sha1: String,
}

fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<File>>>
where
    P: AsRef<Path>,
{
    let file = File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
}

fn parse_xml(
    input: io::Lines<io::BufReader<File>>,
    progressbar: &indicatif::ProgressBar,
) -> Vec<Page> {
    let mut buffer = Vec::<String>::new();
    let mut res = Vec::<Page>::new();
    for line in input.into_iter() {
        if let Ok(l) = line {
            progressbar.inc(l.as_bytes().len() as u64);
            let l = if l.is_empty() {
                continue;
            } else {
                l.trim().to_string()
            };
            let mut xml_reader = Reader::from_str(&l);
            match xml_reader.read_event() {
                Ok(a) => match a {
                    quick_xml::events::Event::Start(bytes_start) => {
                        if (bytes_start.name() == QName(b"mediawiki"))
                            | (bytes_start.name() == QName(b"siteinfo"))
                            | (bytes_start.name() == QName(b"sitename"))
                            | (bytes_start.name() == QName(b"dbname"))
                            | (bytes_start.name() == QName(b"base"))
                            | (bytes_start.name() == QName(b"generator"))
                            | (bytes_start.name() == QName(b"case"))
                            | (bytes_start.name() == QName(b"namespaces"))
                            | (bytes_start.name() == QName(b"namespace"))
                            | (bytes_start.name() == QName(b"generator"))
                        {
                            continue;
                        }
                        if bytes_start.name() == QName(b"page") {
                            buffer.push(l.clone());
                            continue;
                        }
                        let mut s = buffer.pop().unwrap();
                        s.push_str(&l);
                        buffer.push(s);
                    }
                    quick_xml::events::Event::Empty(bytes_start) => {
                        if (bytes_start.name() == QName(b"namespace"))
                            | (bytes_start.name() == QName(b"minor"))
                            | (bytes_start.name() == QName(b"redirect"))
                            | (bytes_start.name() == QName(b"text"))
                        {
                            continue;
                        }
                        todo!("It is assumed that `namespace` and `minor` is the one allowed to be empty: {}", &l);
                    }
                    quick_xml::events::Event::Text(_bytes_text) => {
                        let mut s = buffer.pop().unwrap_or_default();
                        s.push_str(&format!("\n{}", l));
                        buffer.push(s);
                    }
                    quick_xml::events::Event::End(_bytes_end) => {
                        todo!()
                    }
                    quick_xml::events::Event::CData(_bytes_cdata) => todo!(),
                    quick_xml::events::Event::Comment(_bytes_text) => todo!(),
                    quick_xml::events::Event::Decl(_bytes_decl) => todo!(),
                    quick_xml::events::Event::PI(_bytes_pi) => todo!(),
                    quick_xml::events::Event::DocType(_bytes_text) => todo!(),
                    quick_xml::events::Event::Eof => continue,
                },
                Err(quick_xml::Error::IllFormed(IllFormedError::UnmatchedEndTag(a))) => {
                    if (a == "mediawiki")
                        | (a == "siteinfo")
                        | (a == "sitename")
                        | (a == "dbname")
                        | (a == "base")
                        | (a == "generator")
                        | (a == "case")
                        | (a == "namespaces")
                        | (a == "namespace")
                        | (a == "generator")
                    {
                        continue;
                    }
                    let mut s = buffer.pop().unwrap();
                    s.push_str(&l);
                    buffer.push(s);
                    if a == "page" {
                        match quick_xml::de::from_str(&buffer.join("\n")) {
                            Ok(a) => {
                                res.push(a);
                                buffer.clear();
                            }
                            Err(e) => panic!("{}: {}", e, buffer.join("\n")),
                        }
                    }
                }
                Err(_) => continue,
            }
        }
    }
    res
}

use std::{fs::File, io::Read};

use itertools::Itertools;
use markdown::{mdast::Node, Constructs, ParseOptions};
use nom::{
    bytes::complete::tag,
    character::complete::{alphanumeric1, anychar, char, digit0, multispace1, one_of},
    combinator::recognize,
    multi::many0,
    sequence::delimited,
    IResult, Parser as NomParser,
};

#[cfg(test)]
mod test {
    use sqlx::prelude::FromRow;

    #[derive(FromRow)]
    struct TermFreqTable {
        document: Option<String>,
        term: Option<String>,
        lower: Option<String>,
        occurence: Option<i64>,
    }

    #[test]
    fn test_parsers() {
        let mdtext = r#"
Saluang adalah sebuah alat muzik tiup kayu tradisional orang Minangkabau 
dari Sumatra Barat, Indonesia yang mirip dengan seruling pada umumnya 
dan diperbuat dari buluh. Ia berkaitan dengan suling dari bahagian-bahagian 
lain Indonesia.

Alat muzik tiup ini terbuat dari bambu tipis atau talang (Schizostachyum 
brachycladum Kurz); buluh ini merupakan bahan yang lazim digunakan untuk 
membina jemuran kain, dan jenis buluh ini sangat dikehendaki orang Minangkabau 
terutamanya buluh talang yang ditemukan di tepi sungai; malah buluh sama yang 
digunakan untuk memasak lamang juga dianggap sesuai. Alat ini cukup dibuat 
dengan melubangi talang dengan empat lubang. Panjang buluh yang diperlukan 
untuk membuat badan saluang kira-kira 40–60 cm, dengan diameter 3–4 cm. 
Bahagian-bahagian atas dan bawahnya terlebih dahulu untuk menentukan pembuatan 
lubang: bahagian atas saluang ditentukan pada bawah ruas buluh di mana ia diserut 
untuk dibuat meruncing sekitar 45 derajat sesuai ketebalan bambu. Suatu jarak 
2/3 dari panjang bambu diukur dari bahagian atas ditandakan untuk membuat 4 
lubang; jarak antara dua lubang adalah jarak setengah lingkaran bambu. Besar 
lubang agar menghasilkan suara yang bagus disyorkan berdiameter 0.5 sm.

Pemain saluang yang pakar mempunyai kelebihan memainkan saluang dengan meniup 
dan menarik nafas secara serentak sehingga peniup saluang dapat memainkan alat 
musik itu dari awal dari akhir lagu tanpa putus; cara manyisiahan angok ("menyisihkan 
nafas") ini dikembangkan dengan latihan yang terus menerus. Teknik ini dinamakan 
juga sebagai teknik. Tiap nagari di tanah Minangkabau mengembangkan cara meniup 
saluang khas yang tersendiri termasuk di Singgalang, Pariaman, Solok Salayo, 
Koto Tuo, Suayan dan Pauah. Gaya tiupan khas Singgalang dianggap gaya yang 
paling sulit dimahiri pemula, dan biasanya nada Singgalang ini dimainkan 
pada awal lagu, gaya Ratok Solok pula dianggap gaya paling sedih. Pemain 
saluang juga mempunyai mantera tersendiri yang dipercayai berguna untuk memukau 
para pendengar. Mantra itu dinamakan Pitunang Nabi Daud. 
Isi dari mantra itu kira-kira: "Aku malapehan pituang Nabi Daud, buruang 
tabang tatagun-tagun, aia mailia tahanti-hanti, takajuik bidodari di dalam 
sarugo mandanga bunyi saluang ambo, kununlah anak sidang manusia..... 
(Aku melepaskan pitung Nabi Daud, burung terbang tertegun-tegun [terpegun], 
air mengalir terhenti-henti, terkejut bidadari dalam syurga mendengar bunyi 
saluang hamba, kononlah anak sidang manusia...')"
    "#;
        let mdast = markdown::to_mdast(
            &mdtext,
            &markdown::ParseOptions {
                constructs: markdown::Constructs {
                    frontmatter: true,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap();

        let mut collector = Vec::new();
        crate::parser::walk_ast(&mdast, &mut collector);
        assert!(crate::parser::tokenize(&collector).is_ok())
    }

    #[sqlx::test]
    async fn test_sql(pool: sqlx::sqlite::SqlitePool) {
        let _a = sqlx::query! {r#"INSERT INTO term_frequency (document, term, occurence) VALUES (?,?,?)"#, "DOC123", "Term", 1}.execute(&pool).await.unwrap();
        let a: TermFreqTable = sqlx::query_as! {TermFreqTable, r#"SELECT * FROM term_frequency"#}
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(a.document, Some("DOC123".into()));
        assert_eq!(a.term, Some("Term".into()));
        assert_eq!(a.occurence, Some(1))
    }
}

pub(crate) fn tokenize_file(filename: std::path::PathBuf) -> Vec<Token> {
    let mut file = File::open(filename).unwrap();
    let mut buf = String::new();
    file.read_to_string(&mut buf).unwrap();
    let mdast = markdown::to_mdast(
        &buf,
        &ParseOptions {
            constructs: Constructs {
                frontmatter: true,
                ..Default::default()
            },
            ..Default::default()
        },
    )
    .unwrap();
    let mut collector = Vec::new();
    walk_ast(&mdast, &mut collector);
    tokenize(&collector).unwrap()
}
fn match_node(node: &Node, collector: &mut Vec<String>) {
    match node {
        Node::Yaml(_) | Node::Html(_) | Node::Image(_) | Node::InlineCode(_) => {}
        Node::Root(root) => {
            for c in &root.children {
                walk_ast(&c, collector)
            }
        }
        Node::Paragraph(paragraph) => {
            for c in &paragraph.children {
                walk_ast(&c, collector);
            }
        }
        Node::List(list) => {
            for c in &list.children {
                walk_ast(&c, collector);
            }
        }
        Node::Text(text) => {
            collector.push(text.value.to_owned());
        }
        a => todo!("{:?}", a),
    }
}

fn walk_ast(ast: &Node, collector: &mut Vec<String>) {
    match ast.children() {
        Some(nodes) => {
            for node in nodes {
                match_node(node, collector);
            }
        }
        None => match_node(ast, collector),
    }
}

#[derive(Debug, Clone)]
pub(crate) enum Token {
    Text(String),
    Punct(String),
    Unknown(String),
    Omit(String),
}

impl Token {
    pub fn is_text(&self) -> bool {
        match self {
            Token::Text(_) => true,
            _ => false,
        }
    }
}

impl Into<String> for Token {
    fn into(self) -> String {
        match self {
            Token::Text(a) | Token::Punct(a) | Token::Unknown(a) | Token::Omit(a) => a,
        }
    }
}

impl From<String> for Token {
    fn from(value: String) -> Self {
        match (punctuation(&value), known_pattern(&value), numeric(&value)) {
            (Ok(_punct), _, _) => Self::Punct(value),
            (_, Ok(_text), _) => Self::Text(value),
            (_, _, Ok(_digits)) => Self::Omit(value),
            (Err(_), Err(_), Err(_)) => Self::Unknown(value),
        }
    }
}

const PUNCTS: &str = ".,;:–/()\"[]'*|=-{}’%!";
fn punctuation(input: &str) -> IResult<&str, &str> {
    recognize(one_of(PUNCTS)).parse(input)
}
fn kata_ganda(input: &str) -> IResult<&str, &str> {
    recognize(delimited(alphanumeric1, char('-'), alphanumeric1)).parse(input)
}

#[cfg(test)]
#[test]
fn test_markup() {
    let input = r#"{| class="wikitable" ! colspan="2" | Lakonan filemografi"#;
    if let Ok(a) = markup_elem(input) {
        assert!(a.0.trim().is_empty(), "{}", markup_elem(input).unwrap().1);
    }
}
fn numeric(input: &str) -> IResult<&str, &str> {
    recognize(digit0).parse(input)
}
fn markup_elem(input: &str) -> IResult<&str, &str> {
    recognize((tag("{|"), many0(known_pattern))).parse(input)
}
fn known_pattern(input: &str) -> IResult<&str, &str> {
    kata_ganda
        .or(alphanumeric1)
        .or(recognize(multispace1))
        .or(punctuation)
        .parse(input)
}
fn parse(input: &str) -> IResult<&str, Vec<&str>> {
    many0(markup_elem.or(known_pattern).or(recognize(anychar))).parse(input)
}
fn tokenize(collector: &Vec<String>) -> Result<Vec<Token>, String> {
    let input = collector.join(" ");
    let (rest, output) = parse(&input).unwrap();
    if !rest.trim().is_empty() {
        return Err(format!("Unparsed pattern: {}", rest));
    }
    Ok(output
        .iter()
        .filter(|a| !a.trim().is_empty())
        .map(|a| Token::from(a.to_string()))
        .collect())
}

pub(crate) fn ngram2(items: &Vec<Token>) -> Vec<(&String, &String)> {
    let mut res = Vec::new();
    for it in items.iter().tuple_windows::<(_, _)>() {
        match it {
            (Token::Text(a), Token::Text(b)) => res.push((a, b)),
            _ => continue,
        }
    }
    res
}
pub(crate) fn ngram3(items: &Vec<Token>) -> Vec<(&String, &String, &String)> {
    let mut res = Vec::new();
    for it in items.iter().tuple_windows::<(_, _, _)>() {
        match it {
            (Token::Text(a), Token::Text(b), Token::Text(c)) => res.push((a, b, c)),
            _ => continue,
        }
    }
    res
}

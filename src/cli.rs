use clap::{Args, Parser};

#[derive(Args)]
pub(crate) struct Stream {
    #[arg(long)]
    pub root: std::path::PathBuf,
    #[arg(long)]
    pub size: u64,
}

#[derive(Args)]
pub(crate) struct Rank {
    #[arg(long)]
    pub word: String,
}

#[derive(Parser)]
pub(crate) enum Main {
    Stream(Stream),
    Rank(Rank),
}

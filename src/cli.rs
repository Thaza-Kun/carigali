use clap::Parser;

#[derive(Parser)]
pub(crate) struct Args {
    #[arg(long)]
    pub root: std::path::PathBuf,
    #[arg(long)]
    pub size: u64,
    #[deprecated]
    #[arg(long)]
    pub files: std::path::PathBuf,
}

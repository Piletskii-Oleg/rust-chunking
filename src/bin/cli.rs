#[derive(clap::Parser)]
#[command(version, about)]
pub struct Cli {
    #[command(flatten)]
    pub input: Input,

    /// Calculate deduplication ratio
    #[arg(short, long)]
    pub dedup_ratio: bool,

    /// What algorithm to use on the file
    #[arg(value_enum)]
    pub algorithm: Algorithm,
}

#[derive(clap::Args)]
#[group(multiple = false)]
pub struct Input {
    /// Path to the file to be deduplicated
    #[arg(short, long, group = "input")]
    pub path: Option<String>,

    /// Generate data with the given size (in MB) to deduplicate
    #[arg(short, long, group = "gen", value_name = "size")]
    pub generate: Option<usize>,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, clap::ValueEnum)]
pub enum Algorithm {
    Ultra,
    Leap,
}

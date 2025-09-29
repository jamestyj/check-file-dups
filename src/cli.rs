use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "check-file-dups")]
#[command(about = "A CLI tool to find duplicate files in a directory")]
pub struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    pub path: PathBuf,
    
    /// Number of parallel threads for hashing (default: 1)
    #[arg(short, long, default_value = "1")]
    pub threads: Option<usize>,
    
    /// Skip using hash cache and compute all hashes fresh
    #[arg(long)]
    pub no_cache: bool,
}

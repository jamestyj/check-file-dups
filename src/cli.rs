use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "check-file-dups")]
#[command(about = "A CLI tool to find duplicate files in a directory")]
pub struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    pub path: PathBuf,
    
    /// Number of parallel threads for hashing (default: number of CPU cores)
    #[arg(short, long)]
    pub threads: Option<usize>,
    
    /// Skip files smaller than specified size in MB (default: 0)
    #[arg(short, long, default_value = "0")]
    pub min_size: u64,
}

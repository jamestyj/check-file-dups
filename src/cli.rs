use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "check-file-dups")]
#[command(about = "A CLI tool to find duplicate files in a directory")]
pub struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    pub path: PathBuf,

    /// Number of parallel threads for hashing.
    /// Use multiple threads if the images are on NVMe SSD (e.g. CPU is the bottleneck).
    /// Otherwise a single thread (default) is typically faster.
    #[arg(short, long, default_value = "1")]
    pub threads: Option<usize>,

    /// Skip using hash cache and compute all hashes fresh.
    /// For performance testing / benchmarking optimal number of threads to use [default: false]
    #[arg(short, long, default_value = "false")]
    pub no_cache: bool,
}

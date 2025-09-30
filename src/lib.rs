use std::path::PathBuf;

pub mod cache;
pub mod cli;
pub mod duplicates;
pub mod scanner;

pub use cache::HashCache;
pub use cli::Cli;
pub use duplicates::{find_duplicates, print_results};
pub use scanner::{calculate_file_hash, scan_directory_with_cache};

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub hash: String,
}

pub mod cli;
pub mod cache;
pub mod scanner;
pub mod duplicates;
pub mod utils;

pub use cli::Cli;
pub use cache::HashCache;
pub use scanner::{scan_directory_with_cache, calculate_file_hash};
pub use duplicates::{find_duplicates, print_results};
pub use utils::{FileInfo, format_number, format_size};

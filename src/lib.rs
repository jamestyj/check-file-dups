pub mod cache;
pub mod cli;
pub mod duplicates;
pub mod scanner;
pub mod utils;

pub use cache::HashCache;
pub use cli::Cli;
pub use duplicates::{find_duplicates, print_results};
pub use scanner::{calculate_file_hash, scan_directory_with_cache};
pub use utils::FileInfo;

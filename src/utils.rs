use std::path::PathBuf;

pub struct FileInfo {
    pub path: PathBuf,
    pub size: u64,
    pub hash: String,
}

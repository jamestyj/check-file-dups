use anyhow::Result;
use log::info;
use serde_json;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

pub struct HashCache {
    pub cache_file: PathBuf,
    cache: Arc<Mutex<HashMap<String, (u64, String)>>>, // path -> (mtime, hash)
}

impl HashCache {
    pub fn new() -> Self {
        let cache_file = std::env::current_dir()
            .expect("Failed to get current directory")
            .join(format!("{}-cache.json", env!("CARGO_PKG_NAME")));
        let mut cache = HashMap::new();

        // Try to load existing cache
        let cache_size = fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);
        info!(
            "Loading hash cache from: {} ({})",
            cache_file.display(),
            crate::utils::format_size(cache_size)
        );

        // Show a spinner while loading the cache file
        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_message("Loading hash cache...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(loaded_cache) = serde_json::from_str::<HashMap<String, (u64, String)>>(&content) {
                cache = loaded_cache;
            } else {
                info!("No hash cache found, starting fresh");
            }
        }
        spinner.finish_with_message("Hash cache loaded");
        Self {
            cache_file,
            cache: Arc::new(Mutex::new(cache))
        }
    }

    pub fn get_hash(&self, file_path: &PathBuf) -> Result<Option<String>> {
        let path_str = file_path.to_string_lossy().to_string();
        let metadata = file_path.metadata()?;
        let current_mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs();

        if let Ok(cache) = self.cache.lock() {
            if let Some((cached_mtime, cached_hash)) = cache.get(&path_str) {
                if *cached_mtime == current_mtime {
                    return Ok(Some(cached_hash.clone()));
                }
            }
        }
        Ok(None)
    }

    pub fn set_hash(&self, file_path: &PathBuf, hash: String) -> Result<()> {
        let path_str = file_path.to_string_lossy().to_string();
        let metadata = file_path.metadata()?;
        let mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs();

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path_str, (mtime, hash));
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let cache_path = &self.cache_file;
        let cache_size = fs::metadata(cache_path).map(|m| m.len()).unwrap_or(0);
        info!(
            "Saving hash cache to {} ({})",
            cache_path.display(),
            crate::utils::format_size(cache_size)
        );

        // Show a spinner while saving the cache file
        let spinner = indicatif::ProgressBar::new_spinner();
        spinner.set_message("Saving hash cache...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        if let Ok(cache) = self.cache.lock() {
            let content = serde_json::to_string_pretty(&*cache)?;
            fs::write(&self.cache_file, content)?;
        }
        spinner.finish_with_message("Hash cache saved");
        Ok(())
    }
}

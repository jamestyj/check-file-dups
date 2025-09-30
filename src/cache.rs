use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use indicatif::{HumanBytes, HumanCount, ProgressBar};
use log::info;
use serde_json;
use zstd::stream::{decode_all, Encoder};

pub struct HashCache {
    pub cache_file: PathBuf,
    cache: Arc<Mutex<HashMap<String, (u64, u64, String)>>>, // path -> (mtime, size, hash)
}

impl HashCache {
    pub fn new() -> Self {
        let cache_file = std::env::current_dir()
            .expect("Failed to get current directory")
            .join(format!("{}-cache.json.zst", env!("CARGO_PKG_NAME")));
        let mut cache = HashMap::new();

        // Try to load existing cache
        let cache_size = fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);
        info!(
            "Loading hash cache from: {} ({})",
            cache_file.display(),
            HumanBytes(cache_size)
        );

        // Show a spinner while loading the cache file
        let spinner = ProgressBar::new_spinner();
        spinner.set_message("Loading hash cache...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        if let Ok(compressed) = fs::read(&cache_file) {
            if !compressed.is_empty() {
                let mut loaded = None;

                if let Ok(decoded_bytes) = decode_all(&compressed[..]) {
                    // Try new format first (mtime, size, hash)
                    if let Ok(parsed) = serde_json::from_slice::<HashMap<String, (u64, u64, String)>>(&decoded_bytes) {
                        spinner.finish_and_clear();
                        info!("Hash cache has {} entries", HumanCount(parsed.len() as u64));
                        loaded = Some(parsed);
                    } else if let Ok(old_parsed) = serde_json::from_slice::<HashMap<String, (u64, String)>>(&decoded_bytes) {
                        // Migrate from old format (mtime, hash) to new format (mtime, 0, hash)
                        info!("Migrating legacy hash cache format with {} entries", HumanCount(old_parsed.len() as u64));
                        let migrated: HashMap<String, (u64, u64, String)> = old_parsed
                            .into_iter()
                            .map(|(k, (mtime, hash))| (k, (mtime, 0, hash)))
                            .collect();
                        loaded = Some(migrated);
                    } else {
                        info!("Failed to parse decompressed hash cache, falling back");
                    }
                }

                if loaded.is_none() {
                    // Try new format (mtime, size, hash)
                    if let Ok(parsed) = serde_json::from_slice::<HashMap<String, (u64, u64, String)>>(&compressed) {
                        info!("Loaded legacy hash cache (uncompressed) with {} entries", HumanCount(parsed.len() as u64));
                        loaded = Some(parsed);
                    } else if let Ok(old_parsed) = serde_json::from_slice::<HashMap<String, (u64, String)>>(&compressed) {
                        // Migrate from old format
                        info!("Migrating legacy hash cache format (uncompressed) with {} entries", HumanCount(old_parsed.len() as u64));
                        let migrated: HashMap<String, (u64, u64, String)> = old_parsed
                            .into_iter()
                            .map(|(k, (mtime, hash))| (k, (mtime, 0, hash)))
                            .collect();
                        loaded = Some(migrated);
                    } else {
                        info!("Failed to load hash cache, starting fresh");
                    }
                }

                if let Some(parsed) = loaded {
                    cache = parsed;
                }
            }
        }
        spinner.finish_and_clear();
        Self {
            cache_file,
            cache: Arc::new(Mutex::new(cache))
        }
    }

    pub fn get_hash(&self, file_path: &PathBuf) -> Result<Option<String>> {
        let path_str = file_path.to_string_lossy().to_string();
        let metadata = file_path.metadata()?;
        let current_mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        let current_size = metadata.len();

        if let Ok(cache) = self.cache.lock() {
            if let Some((cached_mtime, cached_size, cached_hash)) = cache.get(&path_str) {
                // Cache is valid if both mtime and size match
                if *cached_mtime == current_mtime && *cached_size == current_size {
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
        let size = metadata.len();

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path_str, (mtime, size, hash));
        }
        Ok(())
    }

    pub fn save(&self) -> Result<()> {
        let cache_path = &self.cache_file;
        let cache_size = fs::metadata(cache_path).map(|m| m.len()).unwrap_or(0);
        info!(
            "Saving hash cache to {} ({})",
            cache_path.display(),
            HumanBytes(cache_size)
        );

        // Show a spinner while saving the cache file
        let spinner = ProgressBar::new_spinner();
        spinner.set_message("Saving hash cache...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));
        
        if let Ok(cache) = self.cache.lock() {
            let content = serde_json::to_vec(&*cache)?;
            let file = fs::File::create(&self.cache_file)?;
            let mut encoder = Encoder::new(file, 9)?;
            let threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1);
            if threads > 1 {
                if let Err(err) = encoder.multithread(threads as u32) {
                    info!("Failed to enable multi-threaded compression ({}), using single thread", err);
                }
            }
            encoder.write_all(&content)?;
            encoder.finish()?;
            let new_size = fs::metadata(&self.cache_file).map(|m| m.len()).unwrap_or(0);
            spinner.finish_and_clear();
            info!("Hash cache compressed size: {}", HumanBytes(new_size));
        }
        spinner.finish_and_clear();
        Ok(())
    }
}

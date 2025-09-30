use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use anyhow::Result;
use indicatif::{HumanBytes, HumanCount, ProgressBar};
use log::{info, warn};
use serde_json;
use zstd::stream::{Encoder, decode_all};

/// A thread-safe cache for storing file hash information.
///
/// `HashCache` maintains a mapping from file paths to a tuple containing:
/// - modification time (`mtime`: `u64`)
/// - file size (`size`: `u64`)
/// - hash (`hash`: `String`)
///
/// The cache is protected by a mutex for safe concurrent access, and can be
/// serialized/deserialized to a compressed JSON file on disk.
pub struct HashCache {
    /// Path to the cache file on disk.
    pub cache_file: PathBuf,
    /// The actual cache: path -> (mtime, size, hash).
    cache: Arc<Mutex<HashMap<String, (u64, u64, String)>>>,
}

impl HashCache {
    /// Creates a new `HashCache` instance.
    ///
    /// This function attempts to load a previously saved hash cache from a compressed JSON file
    /// located in the current working directory. The cache file is named using the current
    /// package name and has a `.json.zst` extension (Zstandard-compressed JSON).
    ///
    /// - If the cache file exists:
    ///     - It reads and decompresses the file.
    ///     - It attempts to parse the decompressed data as a `HashMap<String, (u64, u64, String)>`,
    ///       which maps file paths to a tuple of (modification time, file size, hash).
    ///     - If successful, it loads this map into the cache.
    ///     - Progress and status are logged, and a spinner is shown during loading.
    ///     - If parsing fails, a warning is logged and an empty cache is used.
    /// - If the cache file does not exist:
    ///     - A warning is logged and an empty cache is created.
    ///
    /// Returns a `HashCache` struct containing the cache file path and the loaded (or empty) cache,
    /// wrapped in an `Arc<Mutex<...>>` for thread-safe access.
    pub fn new() -> Self {
        let cache_file = std::env::current_dir()
            .expect("Failed to get current directory")
            .join(format!("{}-cache.json.zst", env!("CARGO_PKG_NAME")));
        let mut cache = HashMap::new();

        if let Ok(compressed) = fs::read(&cache_file) {
            let cache_size = fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);
            info!(
                "Loading hash cache from: {} ({})",
                cache_file.display(),
                HumanBytes(cache_size)
            );
            let spinner = ProgressBar::new_spinner();
            spinner.set_message("Loading hash cache...");
            spinner.enable_steady_tick(std::time::Duration::from_millis(100));

            if let Ok(decoded_bytes) = decode_all(&compressed[..]) {
                if let Ok(parsed) =
                    serde_json::from_slice::<HashMap<String, (u64, u64, String)>>(&decoded_bytes)
                {
                    spinner.finish_and_clear();
                    info!("Hash cache has {} entries", HumanCount(parsed.len() as u64));
                    cache = parsed;
                } else {
                    warn!("Failed to parse decompressed hash cache, falling back");
                }
            }
            spinner.finish_and_clear();
        } else {
            warn!("No hash cache file found, starting fresh");
        }
        Self {
            cache_file,
            cache: Arc::new(Mutex::new(cache)),
        }
    }

    /// Retrieves the cached hash for a given file if it is still valid.
    ///
    /// This method normalizes the file path for cross-platform compatibility,
    /// retrieves the file's current metadata (modification time and size),
    /// and checks if there is a cached entry for the file. If a cached entry
    /// exists and both the modification time and file size match the current
    /// file metadata, the cached hash is returned. Otherwise, returns `None`.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file whose hash is being queried.
    /// * `base_path` - The base path to strip from the file path before caching.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(String))` containing the cached hash if valid.
    /// * `Ok(None)` if no valid cache entry exists.
    /// * `Err` if file metadata cannot be accessed.
    pub fn get_hash(&self, file_path: &PathBuf, base_path: &PathBuf) -> Result<Option<String>> {
        // Strip base path and normalize to use forward slashes for cross-platform compatibility
        let relative_path = file_path.strip_prefix(base_path).unwrap_or(file_path);
        let path_str = relative_path.to_string_lossy().replace('\\', "/").trim_start_matches('/').to_string();
        let metadata = file_path.metadata()?;
        let current_mtime = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
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

    /// Updates or inserts the hash for a given file in the cache.
    ///
    /// This method normalizes the file path for cross-platform compatibility,
    /// retrieves the file's metadata (modification time and size), and stores
    /// the hash along with this metadata in the cache. If the file already
    /// exists in the cache, its entry is updated.
    ///
    /// # Arguments
    ///
    /// * `file_path` - The path to the file whose hash is being set.
    /// * `base_path` - The base path to strip from the file path before caching.
    /// * `hash` - The hash string to associate with the file.
    ///
    /// # Errors
    ///
    /// Returns an error if file metadata cannot be accessed.
    pub fn set_hash(&self, file_path: &PathBuf, base_path: &PathBuf, hash: String) -> Result<()> {
        // Strip base path and normalize to use forward slashes for cross-platform compatibility
        let relative_path = file_path.strip_prefix(base_path).unwrap_or(file_path);
        let path_str = relative_path.to_string_lossy().replace('\\', "/").trim_start_matches('/').to_string();
        let metadata = file_path.metadata()?;
        let mtime = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();
        let size = metadata.len();

        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path_str, (mtime, size, hash));
        }
        Ok(())
    }

    /// Saves the current hash cache to disk.
    ///
    /// This method serializes the in-memory hash cache to JSON, compresses it using zstd,
    /// and writes it to the cache file. It displays a spinner while saving and logs the
    /// compressed file size. Use multiple threads for compression if multiple cores are available.
    ///
    /// # Errors
    ///
    /// Returns an error if serialization, file creation, or compression fails.
    pub fn save(&self) -> Result<()> {
        let cache_path = &self.cache_file;
        let cache_size = fs::metadata(cache_path).map(|m| m.len()).unwrap_or(0);
        info!(
            "Saving hash cache to {} ({})",
            cache_path.display(),
            HumanBytes(cache_size)
        );
        let spinner = ProgressBar::new_spinner();
        spinner.set_message("Saving hash cache...");
        spinner.enable_steady_tick(std::time::Duration::from_millis(100));

        if let Ok(cache) = self.cache.lock() {
            let content = serde_json::to_vec(&*cache)?;
            let file = fs::File::create(&self.cache_file)?;
            let mut encoder = Encoder::new(file, 9)?;
            let threads = std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(1);
            if threads > 1 {
                if let Err(err) = encoder.multithread(threads as u32) {
                    info!(
                        "Failed to enable multi-threaded compression ({}), using single thread",
                        err
                    );
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

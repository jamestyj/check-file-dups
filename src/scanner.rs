use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;

use anyhow::Result;
use blake3;
use indicatif::{HumanBytes, HumanCount, ProgressBar, ProgressStyle};
use log::{error, info};
use rayon::prelude::*;
use walkdir::WalkDir;

use crate::cache::HashCache;
use crate::utils::FileInfo;

pub fn calculate_file_hash(file_path: &PathBuf, cache: &HashCache) -> Result<String> {
    // Check cache first
    if let Some(cached_hash) = cache.get_hash(file_path)? {
        return Ok(cached_hash);
    }

    let mut file = fs::File::open(file_path)?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = [0; 8192];

    loop {
        let bytes_read = file.read(&mut buffer)?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    let hash = hasher.finalize().to_hex().to_string();
    
    // Cache the hash
    cache.set_hash(file_path, hash.clone())?;
    
    Ok(hash)
}

pub fn scan_directory_with_cache(
    path: &PathBuf, 
    cache: &HashCache, 
    num_threads: usize
) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    
    // First pass: count files and directories, calculate total size
    let mut total_files = 0;
    let mut total_dirs = 0;
    let mut total_size = 0u64;

    info!("Scanning {}", path.display());
    
    // Add a progress bar for the directory scan
    let pb = ProgressBar::new_spinner();
    pb.set_message("Scanning files and directories...");
    pb.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut file_paths = Vec::new();
    for entry in WalkDir::new(path).into_iter() {
        pb.tick();
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    total_dirs += 1;
                } else if path.is_file() {
                    if let Ok(metadata) = path.metadata() {
                        let size = metadata.len();
                        total_files += 1;
                        total_size += size;
                        file_paths.push(path.to_path_buf());
                    }
                }
            }
            Err(_e) => {
                // error!("Failed to read directory entry: {}", e);
            }
        }
    }
    pb.finish_and_clear();
    
    info!("Found {} files and {} directories ({})", 
    HumanCount(total_files), HumanCount(total_dirs), HumanBytes(total_size));
    
    let progress_bar = {
        let pb = ProgressBar::new(total_size as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg} ETA: {eta}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    };
   
    // Set up parallel processing
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();
    
    let progress_bar = progress_bar.as_ref();
    let files_processed = Arc::new(AtomicUsize::new(0));
    let total_size_processed = Arc::new(AtomicU64::new(0));
    let last_update = Arc::new(std::sync::Mutex::new(std::time::Instant::now()));
    
    // Process files in parallel
    info!("Scanning files...");
    let results: Vec<Result<FileInfo>> = file_paths
        .par_iter()
        .map(|path| {
            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    error!("Failed to read metadata for '{}': {}", path.display(), e);
                    return Err(anyhow::anyhow!("Failed to read metadata"));
                }
            };
            let size = metadata.len();
            
            let hash = match calculate_file_hash(path, &cache) {
                Ok(hash) => hash,
                Err(e) => {
                    error!("Failed to calculate hash for '{}': {}", path.display(), e);
                    return Err(e);
                }
            };
            
            // Update progress
            let processed = files_processed.fetch_add(1, Ordering::Relaxed) + 1;
            let size_processed = total_size_processed.fetch_add(size, Ordering::Relaxed) + size;
            
            if let Some(pb) = progress_bar {
                let mut last_update_guard = last_update.lock().unwrap();
                if last_update_guard.elapsed().as_millis() > 200 {
                    pb.set_position(size_processed as u64);
                    pb.set_message(format!(
                        "Scanned {} files ({})",
                        HumanCount(processed.try_into().unwrap()),
                        HumanBytes(size_processed)
                    ));
                    *last_update_guard = std::time::Instant::now();
                }
            }
            
            Ok(FileInfo {
                path: path.clone(),
                size,
                hash,
            })
        })
        .collect();
    
    // Collect successful results
    for result in results {
        match result {
            Ok(file_info) => files.push(file_info),
            Err(e) => {
                error!("Error processing file: {}", e);
            }
        }
    }
    
    if let Some(pb) = progress_bar {
        pb.finish_with_message("Scan complete!");
    }
    
    Ok(files)
}

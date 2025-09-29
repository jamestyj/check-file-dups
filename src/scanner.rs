use crate::cache::HashCache;
use crate::utils::{FileInfo, format_number, format_size};
use anyhow::Result;
use blake3;
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use rayon::prelude::*;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use walkdir::WalkDir;

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
    num_threads: usize, 
    min_size_mb: u64
) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let walker = WalkDir::new(path).into_iter();
    
    // First pass: count files and directories, calculate total size
    let mut total_files = 0;
    let mut total_dirs = 0;
    let mut total_size = 0u64;

    info!("Scanning: {}", path.display());
    for entry in WalkDir::new(path).into_iter() {
        match entry {
            Ok(entry) => {
                let path = entry.path();
                if path.is_dir() {
                    total_dirs += 1;
                } else if path.is_file() {
                    if let Ok(metadata) = path.metadata() {
                        let size = metadata.len();
                        // Skip files smaller than min_size_mb
                        if min_size_mb == 0 || size >= min_size_mb * 1024 * 1024 {
                            total_files += 1;
                            total_size += size;
                        }
                    }
                }
            }
            Err(e) => {
                error!("Failed to read directory entry: {}", e);
            }
        }
    }
    
    info!("Found {} files and {} directories ({})", 
          format_number(total_files), format_number(total_dirs), format_size(total_size));
    
    let progress_bar = {
        let pb = ProgressBar::new(total_files as u64);
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {percent}% {msg}")
                .unwrap()
                .progress_chars("#>-"),
        );
        Some(pb)
    };
    
    // Collect all file paths first
    let mut file_paths = Vec::new();
    for entry in walker {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                error!("Failed to read directory entry: {}", e);
                continue;
            }
        };
        let path = entry.path();
        
        if path.is_file() {
            // Check file size before adding to processing list
            if let Ok(metadata) = path.metadata() {
                let size = metadata.len();
                // Skip files smaller than min_size_mb
                if min_size_mb == 0 || size >= min_size_mb * 1024 * 1024 {
                    file_paths.push(path.to_path_buf());
                }
            }
        }
    }
    
    // Set up parallel processing
    rayon::ThreadPoolBuilder::new()
        .num_threads(num_threads)
        .build_global()
        .unwrap();
    
    let progress_bar = progress_bar.as_ref();
    let files_processed = Arc::new(AtomicUsize::new(0));
    let total_size_processed = Arc::new(AtomicU64::new(0));
    
    // Process files in parallel
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
                if processed % 100 == 0 || processed == total_files {
                    pb.set_position(processed as u64);
                    pb.set_message(format!(
                        "Scanned {} files ({})",
                        format_number(processed),
                        format_size(size_processed)
                    ));
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

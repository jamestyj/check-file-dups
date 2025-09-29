use clap::Parser;
use blake3::Hasher;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use std::time::Instant;
use walkdir::WalkDir;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use log::{info, warn, error};
use colored::*;
use std::fs;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Parser)]
#[command(name = "check-file-dups")]
#[command(about = "A CLI tool to find duplicate files in a directory")]
struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    path: PathBuf,
    
    /// Enable verbose logging
    #[arg(short, long)]
    verbose: bool,
}

struct FileInfo {
    path: PathBuf,
    size: u64,
    hash: String,
}

struct HashCache {
    cache_file: PathBuf,
    cache: Arc<Mutex<HashMap<String, (u64, String)>>>, // path -> (mtime, hash)
}

impl HashCache {
    fn new() -> Self {
        let cache_file = std::env::temp_dir().join("check-file-dups-cache.json");
        let mut cache = HashMap::new();
        
        // Try to load existing cache
        if let Ok(content) = fs::read_to_string(&cache_file) {
            if let Ok(loaded_cache) = serde_json::from_str::<HashMap<String, (u64, String)>>(&content) {
                cache = loaded_cache;
                let cache_size = fs::metadata(&cache_file).map(|m| m.len()).unwrap_or(0);
                info!(
                    "Loading hash cache from: {} ({} bytes)",
                    cache_file.display(),
                    format_size(cache_size)
                );
            } else {
                info!("No hash cache found, starting fresh");
            }
        }
        
        Self { 
            cache_file, 
            cache: Arc::new(Mutex::new(cache)) 
        }
    }
    
    fn get_hash(&self, file_path: &PathBuf) -> Result<Option<String>> {
        let path_str = file_path.to_string_lossy().to_string();
        
        // Get current file modification time
        let metadata = file_path.metadata()?;
        let current_mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        
        // Check if we have a cached hash for this file
        if let Ok(cache) = self.cache.lock() {
            if let Some((cached_mtime, cached_hash)) = cache.get(&path_str) {
                if *cached_mtime == current_mtime {
                    return Ok(Some(cached_hash.clone()));
                }
            }
        }
        
        Ok(None)
    }
    
    fn set_hash(&self, file_path: &PathBuf, hash: String) -> Result<()> {
        let path_str = file_path.to_string_lossy().to_string();
        let metadata = file_path.metadata()?;
        let mtime = metadata.modified()?.duration_since(std::time::UNIX_EPOCH)?.as_secs();
        
        if let Ok(mut cache) = self.cache.lock() {
            cache.insert(path_str, (mtime, hash));
        }
        Ok(())
    }
    
    fn save(&self) -> Result<()> {
        if let Ok(cache) = self.cache.lock() {
            let content = serde_json::to_string_pretty(&*cache)?;
            fs::write(&self.cache_file, content)?;
        }
        Ok(())
    }
}

fn calculate_file_hash(file_path: &PathBuf, cache: &HashCache) -> Result<String> {
    // Check cache first
    if let Some(cached_hash) = cache.get_hash(file_path)? {
        return Ok(cached_hash);
    }
    
    // Calculate hash
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    
    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0; 8192];

    reader.read(&mut buffer).with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let file_size = file_path.metadata().map(|m| m.len()).unwrap_or(0);
    
    if file_size > 1_073_741_824 {
        info!("Calculating hash for '{}' ({} bytes)", file_path.display(), format_size(file_size));
    }
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    let hash = hasher.finalize().to_hex().to_string();
    
    // Cache the result
    cache.set_hash(file_path, hash.clone())?;
    
    Ok(hash)
}

fn scan_directory_with_cache(path: &PathBuf, cache: &HashCache) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let walker = WalkDir::new(path).into_iter();
    
    // First pass: count files and directories, calculate total size
    let mut total_files = 0;
    let mut total_dirs = 0;
    let mut total_size = 0u64;
    for entry in WalkDir::new(path).into_iter() {
        match entry {
            Ok(entry) => {
                if entry.file_type().is_file() {
                    total_files += 1;
                    if let Ok(metadata) = entry.metadata() {
                        total_size += metadata.len();
                    }
                } else if entry.file_type().is_dir() {
                    total_dirs += 1;
                }
            }
            Err(_) => {
                // Skip errors during counting
            }
        }
    }
    
    info!("Found {} files and {} directories to scan ({})", 
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
    
    let mut files_processed = 0;
    let mut total_size_processed = 0u64;
    let mut last_update = std::time::Instant::now();
    
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
            let metadata = match path.metadata() {
                Ok(metadata) => metadata,
                Err(e) => {
                    error!("Failed to read metadata for '{}': {}", path.display(), e);
                    continue;
                }
            };
            let size = metadata.len();
            
            let hash = match calculate_file_hash(&path.to_path_buf(), &cache) {
                Ok(hash) => hash,
                Err(e) => {
                    error!("{}", e);
                    continue;
                }
            };
            
            files_processed += 1;
            total_size_processed += size;
            files.push(FileInfo {
                path: path.to_path_buf(),
                size,
                hash,
            });
            
            // Update progress bar
            if let Some(pb) = progress_bar.as_ref() {
                pb.inc(1);
                
                // Only update message once per second
                if last_update.elapsed() >= std::time::Duration::from_millis(100) {
                    pb.set_message(format!("Scanned {} files ({} processed)...", 
                                          format_number(files_processed), 
                                          format_size(total_size_processed)));
                    last_update = std::time::Instant::now();
                }
            }
        }
    }
    if let Some(pb) = progress_bar {
        pb.finish_with_message("Scan complete!");
    }
    
    // Save cache
    if let Err(e) = cache.save() {
        error!("Failed to save hash cache: {}", e);
    }
    
    Ok(files)
}

fn find_duplicates(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();
    
    for file in files {
        hash_groups.entry(file.hash.clone()).or_insert_with(Vec::new).push(file);
    }
    
    let total_groups = hash_groups.len();
    
    // Filter out groups with only one file (no duplicates)
    hash_groups.retain(|_, group| group.len() > 1);
    
    let duplicate_groups = hash_groups.len();
    let total_duplicates: usize = hash_groups.values().map(|group| group.len() - 1).sum();
    
    info!("{} unique hashes, {} duplicate groups, {} duplicate files", 
          format_number(total_groups), format_number(duplicate_groups), format_number(total_duplicates));
    
    hash_groups
}

fn format_number(n: usize) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let chars: Vec<char> = s.chars().collect();
    
    for (i, &ch) in chars.iter().enumerate() {
        if i > 0 && (chars.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(ch);
    }
    
    result
}

fn format_size(size: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = size as f64;
    let mut unit_index = 0;
    
    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }
    
    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.1} {}", size, UNITS[unit_index])
    }
}

fn print_results(duplicates: &HashMap<String, Vec<FileInfo>>, base_path: &PathBuf) {
    if duplicates.is_empty() {
        println!("No duplicate files found!");
        return;
    }
    
    let total_duplicates = duplicates.values().map(|group| group.len() - 1).sum::<usize>();
    let total_wasted_space: u64 = duplicates.values()
        .map(|group| group[0].size * (group.len() - 1) as u64)
        .sum();
    
    warn!("Found {} duplicate files wasting {} of space", 
             format_number(total_duplicates), format_size(total_wasted_space));
    
    // Sort duplicate groups by space savings (largest first)
    let mut sorted_groups: Vec<_> = duplicates.into_iter().collect();
    sorted_groups.sort_by(|a, b| {
        let space_a = a.1[0].size * (a.1.len() - 1) as u64;
        let space_b = b.1[0].size * (b.1.len() - 1) as u64;
        space_b.cmp(&space_a) // Reverse order (largest first)
    });
    
    for (_hash, group) in sorted_groups {
        println!("{} ({}, {} files):", "Duplicate group".yellow(), format_size(group[0].size), group.len());
        for file in group {
            // Truncate the base path from the file path
            let relative_path = if file.path.starts_with(base_path) {
                file.path.strip_prefix(base_path).unwrap_or(&file.path)
            } else {
                &file.path
            };
            println!("  {}", relative_path.display());
        }
        println!();
    }
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let cli = Cli::parse();
    
    // Initialize console and file logging
    let log_file = std::env::temp_dir().join("check-file-dups.log");
    
    let log_level = if cli.verbose { simplelog::LevelFilter::Debug } else { simplelog::LevelFilter::Info };
    
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            log_level,
            simplelog::Config::default(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto
        ),
        simplelog::WriteLogger::new(
            log_level,
            simplelog::Config::default(),
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)?
        )
    ])?;
    
    // Log the file location for user reference
    info!("Logs will also be written to: {}", log_file.display());
    
    // Create a global cache instance for signal handling
    let global_cache = Arc::new(HashCache::new());
    let cache_for_signal = global_cache.clone();
    
    // Set up signal handler for Ctrl+C and other unexpected exits
    let running = Arc::new(AtomicBool::new(true));
    let running_for_signal = running.clone();
    
    ctrlc::set_handler(move || {
        info!("Received interrupt signal, saving cache...");
        if let Err(e) = cache_for_signal.save() {
            eprintln!("Failed to save hash cache on exit: {}", e);
        }
        running_for_signal.store(false, Ordering::SeqCst);
        std::process::exit(130); // STATUS_CONTROL_C_EXIT
    })?;
    
    info!("Starting check-file-dups v{}", env!("CARGO_PKG_VERSION"));
    
    if !cli.path.exists() {
        error!("Path does not exist: {}", cli.path.display());
        anyhow::bail!("Path does not exist: {}", cli.path.display());
    }
    
    if !cli.path.is_dir() {
        error!("Path is not a directory: {}", cli.path.display());
        anyhow::bail!("Path is not a directory: {}", cli.path.display());
    }
    
    info!("Scanning: {}", cli.path.display());
    
    let files = scan_directory_with_cache(&cli.path, &global_cache)?;
    
    let duplicates = find_duplicates(files);
    print_results(&duplicates, &cli.path);
    
    let elapsed = start_time.elapsed();
    info!("Program completed successfully in {:.2}s", elapsed.as_secs_f64());
    
    // Final cache save
    if let Err(e) = global_cache.save() {
        error!("Failed to save hash cache on exit: {}", e);
    }
    
    Ok(())
}

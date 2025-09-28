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
use log::{debug, info, error};

#[derive(Parser, Debug)]
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

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    hash: String,
}

fn calculate_file_hash(file_path: &PathBuf) -> Result<String> {
    debug!("Calculating hash for: '{}'", file_path.display());
    
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: '{}'", file_path.display()))?;
    
    let mut reader = BufReader::new(file);
    let mut hasher = Hasher::new();
    let mut buffer = [0; 8192];
    let mut total_bytes = 0;
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .with_context(|| format!("Failed to read file: '{}'", file_path.display()))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
        total_bytes += bytes_read;
    }
    
    let hash = hasher.finalize().to_hex().to_string();
    debug!("Hash calculated for '{}': {} ({} bytes)", file_path.display(), hash, total_bytes);
    
    Ok(hash)
}

fn scan_directory(path: &PathBuf) -> Result<Vec<FileInfo>> {
    info!("Starting directory scan: '{}'", path.display());
    
    let mut files = Vec::new();
    let walker = WalkDir::new(path).into_iter();
    
    let progress_bar = {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message("Scanning files...");
        pb.enable_steady_tick(std::time::Duration::from_secs(1));
        Some(pb)
    };
    
    let mut total_files_found = 0;
    let mut files_processed = 0;
    
    for entry in walker {
        let entry = entry.with_context(|| "Failed to read directory entry")?;
        let path = entry.path();
        
        if path.is_file() {
            total_files_found += 1;
            debug!("Found file: '{}'", path.display());
            
            let metadata = path.metadata()
                .with_context(|| format!("Failed to read metadata for: '{}'", path.display()))?;
            let size = metadata.len();
            
            files_processed += 1;
            let hash = calculate_file_hash(&path.to_path_buf())?;
            files.push(FileInfo {
                path: path.to_path_buf(),
                size,
                hash,
            });
            
            // Update progress bar message with file count
            if let Some(pb) = progress_bar.as_ref() {
                pb.set_message(format!("Scanning files... {} scanned", total_files_found));
            }
        }
    }
    if let Some(pb) = progress_bar {
        pb.finish_with_message("Scan complete!");
    }
    
    info!("Directory scan complete: {} total files, {} processed", 
          total_files_found, files_processed);
    
    Ok(files)
}

fn find_duplicates(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    info!("Analyzing {} files for duplicates", files.len());
    
    let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();
    
    for file in files {
        hash_groups.entry(file.hash.clone()).or_insert_with(Vec::new).push(file);
    }
    
    let total_groups = hash_groups.len();
    
    // Filter out groups with only one file (no duplicates)
    hash_groups.retain(|_, group| group.len() > 1);
    
    let duplicate_groups = hash_groups.len();
    let total_duplicates: usize = hash_groups.values().map(|group| group.len() - 1).sum();
    
    info!("Duplicate analysis complete: {} unique hashes, {} duplicate groups, {} duplicate files", 
          total_groups, duplicate_groups, total_duplicates);
    
    hash_groups
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

fn print_results(duplicates: &HashMap<String, Vec<FileInfo>>) {
    if duplicates.is_empty() {
        println!("No duplicate files found!");
        return;
    }
    
    let total_duplicates = duplicates.values().map(|group| group.len() - 1).sum::<usize>();
    let total_wasted_space: u64 = duplicates.values()
        .map(|group| group[0].size * (group.len() - 1) as u64)
        .sum();
    
    println!("Found {} duplicate files wasting {} of space", 
             total_duplicates, format_size(total_wasted_space));
    
    for (_hash, group) in duplicates {
        println!("Duplicate group ({}):", format_size(group[0].size));
        for file in group {
            println!("  '{}'", file.path.display());
        }
        println!();
    }
}

fn main() -> Result<()> {
    let start_time = Instant::now();
    let cli = Cli::parse();
    
    // Initialize logger with millisecond timestamps
    env_logger::Builder::from_default_env()
        .format_timestamp_millis()
        .filter_level(if cli.verbose { log::LevelFilter::Debug } else { log::LevelFilter::Info })
        .init();
    
    info!("Starting check-file-dups v{}", env!("CARGO_PKG_VERSION"));
    debug!("Command line arguments: {:?}", cli);
    
    // Convert to absolute path for better error messages
    let absolute_path = cli.path.canonicalize()
        .with_context(|| format!("Failed to resolve path: {}", cli.path.display()))?;
    
    if !absolute_path.exists() {
        error!("Path does not exist: {}", absolute_path.display());
        anyhow::bail!("Path does not exist: {}", absolute_path.display());
    }
    
    if !absolute_path.is_dir() {
        error!("Path is not a directory: {}", absolute_path.display());
        anyhow::bail!("Path is not a directory: {}", absolute_path.display());
    }
    
    info!("Target directory: '{}'", absolute_path.display());
    
    info!("Scanning directory: {}", absolute_path.display());
    
    let files = scan_directory(&absolute_path)?;
    info!("Scanned {} files", files.len());
    
    let duplicates = find_duplicates(files);
    print_results(&duplicates);
    
    let elapsed = start_time.elapsed();
    info!("Program completed successfully in {:.2}s", elapsed.as_secs_f64());
    Ok(())
}


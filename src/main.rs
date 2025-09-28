use clap::Parser;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::PathBuf;
use walkdir::WalkDir;
use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

#[derive(Parser)]
#[command(name = "check-file-dups")]
#[command(about = "A CLI tool to find duplicate files in a directory")]
struct Cli {
    /// Directory to scan for duplicates
    #[arg(default_value = ".")]
    path: PathBuf,
    
    /// Show progress bar
    #[arg(short, long)]
    progress: bool,
    
    /// Minimum file size to check (in bytes)
    #[arg(short, long, default_value = "0")]
    min_size: u64,
    
    /// Output format: simple, detailed, json
    #[arg(short, long, default_value = "simple")]
    format: String,
}

#[derive(Debug)]
struct FileInfo {
    path: PathBuf,
    size: u64,
    hash: String,
}

fn calculate_file_hash(file_path: &PathBuf) -> Result<String> {
    let file = File::open(file_path)
        .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
    
    let mut reader = BufReader::new(file);
    let mut hasher = Sha256::new();
    let mut buffer = [0; 8192];
    
    loop {
        let bytes_read = reader.read(&mut buffer)
            .with_context(|| format!("Failed to read file: {}", file_path.display()))?;
        
        if bytes_read == 0 {
            break;
        }
        
        hasher.update(&buffer[..bytes_read]);
    }
    
    Ok(format!("{:x}", hasher.finalize()))
}

fn scan_directory(path: &PathBuf, min_size: u64, show_progress: bool) -> Result<Vec<FileInfo>> {
    let mut files = Vec::new();
    let walker = WalkDir::new(path).into_iter();
    
    let progress_bar = if show_progress {
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"]),
        );
        pb.set_message("Scanning files...");
        Some(pb)
    } else {
        None
    };
    
    for entry in walker {
        let entry = entry?;
        let path = entry.path();
        
        if path.is_file() {
            let metadata = path.metadata()?;
            let size = metadata.len();
            
            if size >= min_size {
                let hash = calculate_file_hash(&path.to_path_buf())?;
                files.push(FileInfo {
                    path: path.to_path_buf(),
                    size,
                    hash,
                });
            }
        }
    }
    
    if let Some(pb) = progress_bar {
        pb.finish_with_message("Scan complete!");
    }
    
    Ok(files)
}

fn find_duplicates(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();
    
    for file in files {
        hash_groups.entry(file.hash.clone()).or_insert_with(Vec::new).push(file);
    }
    
    // Filter out groups with only one file (no duplicates)
    hash_groups.retain(|_, group| group.len() > 1);
    
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

fn print_results(duplicates: &HashMap<String, Vec<FileInfo>>, format: &str) {
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
    println!();
    
    match format {
        "detailed" => {
            for (_hash, group) in duplicates {
                println!("Hash: {}", _hash);
                println!("Size: {}", format_size(group[0].size));
                println!("Files:");
                for file in group {
                    println!("  {}", file.path.display());
                }
                println!();
            }
        }
        "json" => {
            println!("{{");
            println!("  \"duplicates\": [");
            for (i, (_hash, group)) in duplicates.iter().enumerate() {
                println!("    {{");
                println!("      \"hash\": \"{}\",", _hash);
                println!("      \"size\": {},", group[0].size);
                println!("      \"files\": [");
                for (j, file) in group.iter().enumerate() {
                    println!("        \"{}\"{}", 
                             file.path.display().to_string().replace('\\', "/"),
                             if j < group.len() - 1 { "," } else { "" });
                }
                println!("      ]");
                println!("    }}{}", if i < duplicates.len() - 1 { "," } else { "" });
            }
            println!("  ]");
            println!("}}");
        }
        _ => { // simple format
            for (_hash, group) in duplicates {
                println!("Duplicate group ({}):", format_size(group[0].size));
                for file in group {
                    println!("  {}", file.path.display());
                }
                println!();
            }
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    if !cli.path.exists() {
        anyhow::bail!("Path does not exist: {}", cli.path.display());
    }
    
    if !cli.path.is_dir() {
        anyhow::bail!("Path is not a directory: {}", cli.path.display());
    }
    
    println!("Scanning directory: {}", cli.path.display());
    if cli.min_size > 0 {
        println!("Minimum file size: {}", format_size(cli.min_size));
    }
    println!();
    
    let files = scan_directory(&cli.path, cli.min_size, cli.progress)?;
    println!("Scanned {} files", files.len());
    
    let duplicates = find_duplicates(files);
    print_results(&duplicates, &cli.format);
    
    Ok(())
}


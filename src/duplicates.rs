use std::collections::HashMap;
use std::path::PathBuf;

use colored::Colorize;
use indicatif::{HumanBytes, HumanCount};
use log::{info, warn};

use crate::FileInfo;

pub fn find_duplicates(files: Vec<FileInfo>) -> HashMap<String, Vec<FileInfo>> {
    let mut hash_groups: HashMap<String, Vec<FileInfo>> = HashMap::new();
    
    info!("Finding duplicates...");
    for file in files {
        let hash = file.hash.clone();
        hash_groups.entry(hash).or_insert_with(Vec::new).push(file);
    }
    
    // Filter out groups with only one file (no duplicates)
    hash_groups.retain(|_, group| group.len() > 1);
    
    hash_groups
}

pub fn print_results(duplicates: &HashMap<String, Vec<FileInfo>>, base_path: &PathBuf) {
    if duplicates.is_empty() {
        println!("{}", "No duplicate files found!".green());
        return;
    }
    let total_duplicates = duplicates.values().map(|group| group.len() - 1).sum::<usize>();
    let total_wasted_space: u64 = duplicates.values()
        .map(|group| group[0].size * (group.len() - 1) as u64)
        .sum();
    
    warn!("Found {} duplicate files wasting {} of space", 
        HumanCount(total_duplicates.try_into().unwrap()), HumanBytes(total_wasted_space));
    
    // Sort duplicate groups by space savings (largest first)
    let mut sorted_groups: Vec<_> = duplicates.into_iter().collect();
    sorted_groups.sort_by(|a, b| {
        let space_a = a.1[0].size * (a.1.len() - 1) as u64;
        let space_b = b.1[0].size * (b.1.len() - 1) as u64;
        space_b.cmp(&space_a) // Reverse order (largest first)
    });
    
    for (_hash, group) in sorted_groups {
        warn!("Duplicate group ({}, {} files):", HumanBytes(group[0].size), group.len());
        for file in group {
            // Truncate the base path from the file path
            let relative_path = if file.path.starts_with(base_path) {
                file.path.strip_prefix(base_path).unwrap_or(&file.path)
            } else {
                &file.path
            };
            warn!("  {}", relative_path.display());
        }
    }
}

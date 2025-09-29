use anyhow::Result;
use clap::Parser;
use indicatif::HumanDuration;
use log::{error, info};
use simplelog::{ColorChoice, CombinedLogger, ConfigBuilder, LevelFilter, TerminalMode, TermLogger, WriteLogger};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use time::macros::format_description;

use check_file_dups::{Cli, HashCache, find_duplicates, print_results, scan_directory_with_cache};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start_time = std::time::Instant::now();
    
    // Initialize console and file logging
    let log_file = std::env::current_dir()?.join(format!("{}.log", env!("CARGO_PKG_NAME")));
    let log_level = LevelFilter::Info;
    let log_config = ConfigBuilder::new()
        .set_time_format_custom(format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"))
        .build();
    CombinedLogger::init(vec![
        TermLogger::new(
            log_level,
            log_config.clone(),
            TerminalMode::Mixed,
            ColorChoice::Auto
        ),
        WriteLogger::new(
            log_level,
            log_config,
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)?
        )
    ])?;

    info!(
        "Starting check-file-dups v{} with options: path={}, threads={:?}, no_cache={}",
        env!("CARGO_PKG_VERSION"),
        cli.path.display(),
        cli.threads.unwrap(),
        cli.no_cache
    );
    info!("Logging to {}", log_file.display());
    
    if cli.no_cache {
        info!("Hash cache disabled - computing all hashes fresh");
    }
    
    // Create a global cache instance for signal handling
    let global_cache = Arc::new(HashCache::new());
    let cache_for_signal = global_cache.clone();

    // Set up signal handler for Ctrl+C and other unexpected exits
    let running = Arc::new(AtomicBool::new(true)); // Not directly used for loop control yet
    let running_for_signal = running.clone();

    let no_cache_for_signal = cli.no_cache;
    ctrlc::set_handler(move || {
        if !no_cache_for_signal {
            info!("Received interrupt signal, saving cache...");
            if let Err(e) = cache_for_signal.save() {
                eprintln!("Failed to save hash cache on exit: {}", e);
            }
        } else {
            info!("Received interrupt signal, exiting...");
        }
        running_for_signal.store(false, Ordering::SeqCst);
        std::process::exit(130); // STATUS_CONTROL_C_EXIT
    })?;

    let files = scan_directory_with_cache(&cli.path, &global_cache, cli.threads.unwrap(), cli.no_cache)?;

    let duplicates = find_duplicates(files);
    print_results(&duplicates, &cli.path);

    // Final cache save (only if caching is enabled)
    if !cli.no_cache {
        if let Err(e) = global_cache.save() {
            error!("Failed to save hash cache on exit: {}", e);
        }
    }

    info!("Program completed successfully in {}", HumanDuration(start_time.elapsed()));

    Ok(())
}

use check_file_dups::*;
use clap::Parser;
use anyhow::Result;
use log::info;
use simplelog;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use time::macros::format_description;

fn main() -> Result<()> {
    let cli = Cli::parse();
    let start_time = std::time::Instant::now();
    
    // Initialize console and file logging
    let log_file = std::env::current_dir()?.join(format!("{}.log", env!("CARGO_PKG_NAME")));
    let log_level = simplelog::LevelFilter::Info;
    let log_config = simplelog::ConfigBuilder::new()
        .set_time_format_custom(format_description!("[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:3]"))
        .build();
    simplelog::CombinedLogger::init(vec![
        simplelog::TermLogger::new(
            log_level,
            log_config.clone(),
            simplelog::TerminalMode::Mixed,
            simplelog::ColorChoice::Auto
        ),
        simplelog::WriteLogger::new(
            log_level,
            log_config,
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)?
        )
    ])?;

    info!(
        "Starting check-file-dups v{} with options: path={}, threads={:?}",
        env!("CARGO_PKG_VERSION"),
        cli.path.display(),
        cli.threads.unwrap()
    );
    info!("Logging to {}", log_file.display());
    
    // Create a global cache instance for signal handling
    let global_cache = Arc::new(HashCache::new());
    let cache_for_signal = global_cache.clone();

    // Set up signal handler for Ctrl+C and other unexpected exits
    let running = Arc::new(AtomicBool::new(true)); // Not directly used for loop control yet
    let running_for_signal = running.clone();

    ctrlc::set_handler(move || {
        info!("Received interrupt signal, saving cache...");
        if let Err(e) = cache_for_signal.save() {
            eprintln!("Failed to save hash cache on exit: {}", e);
        }
        running_for_signal.store(false, Ordering::SeqCst);
        std::process::exit(130); // STATUS_CONTROL_C_EXIT
    })?;

    let files = scan_directory_with_cache(&cli.path, &global_cache, cli.threads.unwrap())?;

    let duplicates = find_duplicates(files);
    print_results(&duplicates, &cli.path);

    // Final cache save
    if let Err(e) = global_cache.save() {
        log::error!("Failed to save hash cache on exit: {}", e);
    }

    let elapsed = start_time.elapsed();
    info!("Program completed successfully in {:.2}s", elapsed.as_secs_f64());

    Ok(())
}

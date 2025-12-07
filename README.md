# Check File Dups

A fast and efficient Rust CLI tool to find duplicate files in a directory using BLAKE3 hashing with intelligent caching and parallel processing.

## Features

- **Fast file hashing**: Uses BLAKE3 for high-performance duplicate detection
- **Intelligent caching**: Saves computed hashes to avoid recomputation on subsequent runs
- **Parallel processing**: Multi-threaded file processing for maximum performance
- **Recursive directory scanning**: Scans all subdirectories with progress tracking
- **Space calculation**: Shows how much space duplicates are wasting
- **Colored output**: Green success message when no duplicates are found
- **Comprehensive logging**: Console and file logging with timestamps
- **Graceful exit**: Saves cache on Ctrl+C and unexpected exits

## Performance Features

- **BLAKE3 hashing**: Fast cryptographic hashing optimized for speed
- **Hash caching**: 10x speedup on subsequent runs by caching computed hashes
- **Parallel processing**: Multi-threaded file processing with configurable thread count
- **Efficient I/O**: 8KB buffer reads for optimal disk performance
- **Progress tracking**: Real-time progress bar with file count and size information
- **Memory efficient**: Streams large files without loading them entirely into memory

## Cache System

The tool automatically creates a hash cache file (`check-file-dups-cache.json`) in the current directory:

- **Purpose**: Stores file hashes with modification times to avoid recomputation
- **Persistence**: Cache is saved on normal exit and on Ctrl+C interruption
- **Validation**: Files are re-hashed if their modification time changes

## Logging

- **Console output**: Real-time progress and results
- **File logging**: Detailed logs saved to `check-file-dups.log` in the current directory
- **Timestamp format**: `YYYY-MM-DD HH:MM:SS.mmm` with millisecond precision
- **Log levels**: INFO for general operations, WARN for duplicate findings

## Requirements

- Rust 1.90 or later
- Windows, macOS, or Linux
- Sufficient disk space for hash cache (typically 1-2% of scanned data size)

## Code Organization

The code is organized into the following files:

- `cache.rs`: Hash caching system
- `cli.rs`: Command-line argument parsing
- `duplicates.rs`: Duplicate detection and result formatting
- `lib.rs`: Utility functions and data structures
- `main.rs`: Application orchestration
- `scanner.rs`: File scanning and hashing logic

## Installation

### From Source

1. Install Rust: <https://rust-lang.org/tools/install>.
2. Clone this repository. For example:

    ```term
    > git clone git@github.com:jamestyj/check-file-dups.git
    > cd check-file-dups
    ```

3. Build the project.

    ```term
    > cargo build --release
    ```

    Sample output:

    ```term
    > cargo build --release
    Compiling getrandom v0.3.3
    Compiling proc-macro2 v1.0.101
    ...
    Compiling check-file-dups v0.1.0 (C:\code\check-file-dups)
     Finished `release` profile [optimized] target(s) in 8.02s
    ```

## Usage

### Display help

Run with `--help` to display command arguments and options. For example:

```term
> .\target\release\check-file-dups --help
A CLI tool to find duplicate files in a directory

Usage: check-file-dups.exe [OPTIONS] [PATH]

Arguments:
  [PATH]  Directory to scan for duplicates [default: .]

Options:
  -t, --threads <THREADS>  Number of parallel threads for hashing. Use multiple threads if the
                           images are on NVMe SSD (e.g. CPU is the bottleneck). Otherwise a
                           single thread (default) is typically faster [default: 1]
  -n, --no-cache           Skip using hash cache and compute all hashes fresh. For performance
                           testing / benchmarking optimal number of threads to use [default: false]
  -h, --help               Print help
```

### Configuration file

To configure the tool, copy [`check-file-dups.example.toml`](./check-file-dups.example.toml) to `check-file-dups.toml` and customize it.

```toml
# check-file-dups configuration file
#
# Copy this file to check-file-dups.toml to customize behavior

# base_path: The base path to strip from the file paths in output.
# Useful if scanning a mounted drive or specific subdirectory.
# Example: base_path = "C:\\path\\to\\scan"
base_path = ""

# skip_dirs: List of directory names or paths to skip during scanning.
# Example: skip_dirs = ["@eaDir", "Lightroom Backups"]
skip_dirs = []
```

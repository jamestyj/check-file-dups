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

## Installation

### From Source

```bash
git clone <repository-url>
cd check-file-dups
cargo build --release
```

## Usage

```bash
# Scan current directory
./target/release/check-file-dups

# Scan specific directory
./target/release/check-file-dups /path/to/directory

# Use custom thread count
./target/release/check-file-dups --threads 8 /path/to/directory
```

## Command Line Options

- `path`: Directory to scan (default: current directory)
- `-t, --threads <THREADS>`: Number of parallel threads for hashing (default: 1)
- `-h, --help`: Print help information

## Output

### When duplicates are found:
```
Found 5 duplicate files wasting 2.3 MB of space

Duplicate group (1.2 MB, 2 files):
  file1.txt
  file2.txt

Duplicate group (800 KB, 3 files):
  photo1.jpg
  photo1_copy.jpg
  photo1_backup.jpg
```

### When no duplicates are found:
```
No duplicate files found!
```
*(Displayed in green)*

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

## Architecture

The project is organized into modular components:
- `cli.rs`: Command-line argument parsing
- `scanner.rs`: File scanning and hashing logic
- `cache.rs`: Hash caching system
- `duplicates.rs`: Duplicate detection and result formatting
- `utils.rs`: Utility functions and data structures
- `main.rs`: Application orchestration

## Requirements

- Rust 1.90 or later
- Windows, macOS, or Linux
- Sufficient disk space for hash cache (typically 1-2% of scanned data size)

## TODOs

- Compile and run on NAS (avoids slow network transfer)
- Try xxHash instead of Blake3 (especially when on NAS)
- Add option to delete duplicate files + dry-run mode
  - Delete duplicate with longer filename
- Check for original and -edited version, delete original

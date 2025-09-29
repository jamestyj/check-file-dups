# Check File Dups

A fast and efficient Rust CLI tool to find duplicate files in a directory using SHA-256 hashing.

## TODOs
- Compile and run on NAS (avoids slow network transfer)
- Try xxHash instead of Blake3 (especially when on NAS)
- Add option to delete duplicate files + dry-run mode
  - Delete duplicate with longer filename
- Check for original and -edited version, delete original

## Features

- **Fast file hashing**: Uses BLAKE3 to identify duplicate files
- **Recursive directory scanning**: Scans all subdirectories
- **Space calculation**: Shows how much space duplicates are wasting

## Installation

### From Source

```bash
git clone <repository-url>
cd check-file-dups
cargo build --release
```

## Usage

```bash
# Scan specific directory
./target/release/check-file-dups /path/to/directory
```

## Command Line Options

- `path`: Directory to scan (default: current directory)

## Output
```
Found 5 duplicate files wasting 2.3 MB of space

Duplicate group (1.2 MB):
  /path/to/file1.txt
  /path/to/file2.txt
```

## Performance

The tool is optimized for performance:
- Uses efficient file reading with 8KB buffers
- Blake3 hashing for fast and reliable duplicate detection
- Memory-efficient processing of large directories

## Requirements

- Rust 1.90 or later
- Windows, macOS, or Linux


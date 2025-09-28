# Check File Dups

A fast and efficient Rust CLI tool to find duplicate files in a directory using SHA-256 hashing.

## Features

- **Fast file hashing**: Uses SHA-256 to identify duplicate files
- **Recursive directory scanning**: Scans all subdirectories
- **Progress bar**: Optional progress indicator for large directories
- **Multiple output formats**: Simple, detailed, and JSON output
- **Size filtering**: Skip files smaller than a specified size
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
# Scan current directory
./target/release/check-file-dups

# Scan specific directory
./target/release/check-file-dups /path/to/directory

# Show progress bar
./target/release/check-file-dups --progress

# Skip files smaller than 1MB
./target/release/check-file-dups --min-size 1048576

# Output in detailed format
./target/release/check-file-dups --format detailed

# Output in JSON format
./target/release/check-file-dups --format json
```

## Command Line Options

- `path`: Directory to scan (default: current directory)
- `--progress, -p`: Show progress bar during scanning
- `--min-size, -s`: Minimum file size to check in bytes (default: 0)
- `--format, -f`: Output format - simple, detailed, or json (default: simple)

## Output Formats

### Simple (default)
```
Found 5 duplicate files wasting 2.3 MB of space

Duplicate group (1.2 MB):
  /path/to/file1.txt
  /path/to/file2.txt
```

### Detailed
```
Found 5 duplicate files wasting 2.3 MB of space

Hash: a1b2c3d4e5f6...
Size: 1.2 MB
Files:
  /path/to/file1.txt
  /path/to/file2.txt
```

### JSON
```json
{
  "duplicates": [
    {
      "hash": "a1b2c3d4e5f6...",
      "size": 1258291,
      "files": [
        "/path/to/file1.txt",
        "/path/to/file2.txt"
      ]
    }
  ]
}
```

## Performance

The tool is optimized for performance:
- Uses efficient file reading with 8KB buffers
- SHA-256 hashing for reliable duplicate detection
- Memory-efficient processing of large directories
- Optional progress indication for long-running scans

## Requirements

- Rust 1.70 or later
- Windows, macOS, or Linux


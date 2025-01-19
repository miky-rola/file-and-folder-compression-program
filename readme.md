# Rust File Compressor

A command-line utility written in Rust that provides file compression and decompression functionality using run-length encoding (RLE).

## Features

- Interactive command-line interface
- Compression of single files and directories
- Decompression of archived files
- Recursive file search functionality
- File size and modification time display
- Support for handling existing files
- Progress tracking during compression/decompression

## Installation

1. Make sure you have Rust and Cargo installed. If not, install them from [rustup.rs](https://rustup.rs/)

2. Clone the repository:
```bash
git clone https://github.com/miky-rola/file-and-folder-compression-program
cd rust-file-compressor
```

3. Build the project:
```bash
cargo build --release
```

The compiled binary will be available in `target/release/`

## Usage

Run the program:
```bash
cargo run
```

### Available Commands

- `compress <filename/foldername>` - Compress a file or folder
- `decompress <archive_name>` - Extract a compressed archive
- `help` - Show available commands
- `exit` - Exit the program

### Example Usage

1. Compressing a file:
```bash
compressor> compress myfile.txt
```

2. Compressing a directory:
```bash
compressor> compress mydirectory
```

3. Decompressing an archive:
```bash
compressor> decompress myfile.compressed
```

## Project Structure

- `src/main.rs` - Program entry point
- `src/models.rs` - Data structures for file entries and information
- `src/utils.rs` - Utility functions for formatting
- `src/compressor.rs` - Core compression/decompression logic
- `src/shell.rs` - Interactive command-line interface implementation

## Technical Details

### Compression Algorithm

The project uses a simple run-length encoding (RLE) compression algorithm:
- Sequences of repeated bytes are encoded as (byte, count) pairs
- Each encoded pair uses 5 bytes (1 for the symbol, 4 for the count)
- The archive format includes a header with file information and offsets

### File Format

Compressed archives have the following structure:
1. Number of files (8 bytes)
2. For each file:
   - Path length (4 bytes)
   - Path string (variable length)
   - Compressed size (8 bytes)
   - File offset (8 bytes)
3. Compressed file data

## Dependencies

- `chrono` - For date/time formatting

## Contributing

1. Fork the repository
2. Create a feature branch
3. Commit your changes
4. Push to the branch
5. Create a Pull Request


## Author

miky rola
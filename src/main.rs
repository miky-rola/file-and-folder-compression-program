use std::collections::HashMap;
use std::fs::{self, File, DirEntry};
use std::io::{self, Read, Write, BufReader, BufWriter, Seek, SeekFrom};
use std::path::{Path, PathBuf};

#[derive(Debug)]
struct FileEntry {
    path: String,
    size: u64,
    offset: u64,
}

struct Compressor {
    input_path: String,
    output_path: String,
}

impl Compressor {
    fn new(input_path: String, output_path: String) -> Self {
        Compressor {
            input_path,
            output_path,
        }
    }

    fn format_size(size: u64) -> String {
        const KB: u64 = 1024;
        const MB: u64 = KB * 1024;
        const GB: u64 = MB * 1024;

        if size >= GB {
            format!("{:.2} GB", size as f64 / GB as f64)
        } else if size >= MB {
            format!("{:.2} MB", size as f64 / MB as f64)
        } else if size >= KB {
            format!("{:.2} KB", size as f64 / KB as f64)
        } else {
            format!("{} B", size)
        }
    }

    fn format_time(time: SystemTime) -> String {
        time.duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| {
                let secs = d.as_secs();
                let naive = chrono::NaiveDateTime::from_timestamp_opt(secs as i64, 0)
                    .unwrap_or_default();
                naive.format("%Y-%m-%d %H:%M:%S").to_string()
            })
            .unwrap_or_else(|_| String::from("Unknown"))
    }


    fn compress_file(reader: &mut dyn Read, writer: &mut dyn Write) -> io::Result<u64> {
        let mut current_byte = None;
        let mut count: i32 = 0;
        let mut buffer = [0; 1];
        let mut bytes_written = 0;

        while reader.read_exact(&mut buffer).is_ok() {
            match current_byte {
                None => {
                    current_byte = Some(buffer[0]);
                    count = 1;
                }
                Some(byte) if byte == buffer[0] => {
                    count += 1;
                }
                Some(byte) => {
                    // Write the current run
                    writer.write_all(&[byte])?;
                    writer.write_all(&count.to_le_bytes())?;
                    bytes_written += 5; // 1 byte for symbol + 4 bytes for count
                    
                    current_byte = Some(buffer[0]);
                    count = 1;
                }
            }
        }

        // Write the last run if any
        if let Some(byte) = current_byte {
            writer.write_all(&[byte])?;
            writer.write_all(&count.to_le_bytes())?;
            bytes_written += 5;
        }

        Ok(bytes_written)
    }

    fn collect_files(dir: &Path) -> io::Result<Vec<PathBuf>> {
        let mut files = Vec::new();
        if dir.is_file() {
            files.push(dir.to_path_buf());
            return Ok(files);
        }

        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            } else if path.is_dir() {
                files.extend(Self::collect_files(&path)?);
            }
        }
        Ok(files)
    }

    fn handle_existing_file(path: &str) -> io::Result<bool> {
        if Path::new(path).exists() {
            println!("\nFile '{}' already exists!", path);
            println!("Choose an option:");
            println!("1. Replace existing file");
            println!("2. Create new file with different name");
            
            let mut choice = String::new();
            io::stdin().read_line(&mut choice)?;
            
            match choice.trim() {
                "1" => Ok(true),
                "2" => {
                    println!("Enter new file name:");
                    let mut new_name = String::new();
                    io::stdin().read_line(&mut new_name)?;
                    Err(io::Error::new(io::ErrorKind::AlreadyExists, new_name.trim().to_string()))
                },
                _ => {
                    println!("Invalid choice. Operation cancelled.");
                    Err(io::Error::new(io::ErrorKind::InvalidInput, "Operation cancelled"))
                }
            }
        } else {
            Ok(false)
        }
    }

    fn compress(&self) -> io::Result<()> {
        // Handle existing file
        match Self::handle_existing_file(&self.output_path) {
            Ok(true) => {}, // Replace existing file
            Ok(false) => {}, // New file
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // Create new file with different name
                let new_path = e.to_string();
                return Compressor::new(self.input_path.clone(), new_path).compress();
            }
            Err(e) => return Err(e),
        }

        let input_path = Path::new(&self.input_path);
        let output_file = File::create(&self.output_path)?;
        let mut writer = BufWriter::new(output_file);

        // Collect all files to compress
        let files = Self::collect_files(input_path)?;
        
        // Write number of files
        writer.write_all(&(files.len() as u64).to_le_bytes())?;
        
        // Calculate header size - convert all values to u64 first
        let header_pos = (files.len() as u64) * 
            ((std::mem::size_of::<u64>() as u64 * 2) + // size and offset
             (std::mem::size_of::<u32>() as u64)); // path length
        let mut current_offset = header_pos;
        
        let mut file_entries = Vec::new();

        // First pass: compress files and collect metadata
        for file_path in &files {
            println!("Compressing: {}", file_path.display());
            let relative_path = file_path.strip_prefix(input_path)
                .unwrap_or(file_path)
                .to_string_lossy()
                .into_owned();

            let mut input_file = File::open(file_path)?;
            let mut temp_output = Vec::new();
            
            let compressed_size = Self::compress_file(&mut input_file, &mut temp_output)?;
            
            writer.write_all(&temp_output)?;
            
            file_entries.push(FileEntry {
                path: relative_path,
                size: compressed_size,
                offset: current_offset,
            });
            
            current_offset += compressed_size;
        }

        // Write file entries at the beginning
        writer.seek(SeekFrom::Start(8))?; // Skip file count
        for entry in file_entries {
            // Write path
            let path_bytes = entry.path.as_bytes();
            writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(path_bytes)?;
            
            // Write metadata
            writer.write_all(&entry.size.to_le_bytes())?;
            writer.write_all(&entry.offset.to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }

    fn decompress(&self) -> io::Result<()> {
        let input_file = File::open(&self.input_path)?;
        let mut reader = BufReader::new(input_file);
        
        // Read number of files
        let mut count_buffer = [0; 8];
        reader.read_exact(&mut count_buffer)?;
        let num_files = u64::from_le_bytes(count_buffer);
        
        // Read file entries
        let mut files = Vec::new();
        for _ in 0..num_files {
            // Read path length
            let mut len_buffer = [0; 4];
            reader.read_exact(&mut len_buffer)?;
            let path_len = u32::from_le_bytes(len_buffer) as usize;
            
            // Read path
            let mut path_buffer = vec![0; path_len];
            reader.read_exact(&mut path_buffer)?;
            let path = String::from_utf8(path_buffer).unwrap();
            
            // Read metadata
            let mut size_buffer = [0; 8];
            let mut offset_buffer = [0; 8];
            reader.read_exact(&mut size_buffer)?;
            reader.read_exact(&mut offset_buffer)?;
            
            files.push(FileEntry {
                path,
                size: u64::from_le_bytes(size_buffer),
                offset: u64::from_le_bytes(offset_buffer),
            });
        }

        // Create output directory if it doesn't exist
        let output_dir = Path::new(&self.output_path);
        fs::create_dir_all(output_dir)?;

        // Decompress each file
        for file_entry in files {
            let output_path = output_dir.join(&file_entry.path);
            println!("Extracting: {}", output_path.display());
            
            // Create parent directories if they don't exist
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            let output_file = File::create(output_path)?;
            let mut writer = BufWriter::new(output_file);

            // Seek to file data
            reader.seek(SeekFrom::Start(file_entry.offset))?;
            
            let mut bytes_read = 0;
            while bytes_read < file_entry.size {
                let mut symbol_buffer = [0; 1];
                let mut count_buffer = [0; 4];
                
                reader.read_exact(&mut symbol_buffer)?;
                reader.read_exact(&mut count_buffer)?;
                
                let count = u32::from_le_bytes(count_buffer);
                for _ in 0..count {
                    writer.write_all(&[symbol_buffer[0]])?;
                }
                
                bytes_read += 5; // 1 byte symbol + 4 bytes count
            }
            writer.flush()?;
        }

        Ok(())
    }
}

fn print_help() {
    println!("\nAvailable commands:");
    println!("  compress <source_path> <output_path>  - Compress a file or folder");
    println!("  decompress <archive_path> <output_dir> - Extract compressed archive");
    println!("  help                                  - Show this help message");
    println!("  exit                                  - Exit the program");
    println!("\nExamples:");
    println!("  compress /path/to/folder archive.bin");
    println!("  decompress archive.bin /path/to/extract");
}

fn run_shell() -> io::Result<()> {
    println!("Welcome to Rust File Compressor!");
    println!("Type 'help' for available commands");

    loop {
        print!("\ncompressor> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;

        let args: Vec<&str> = input.trim().split_whitespace().collect();
        if args.is_empty() {
            continue;
        }

        match args[0] {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                print_help();
            }
            "compress" => {
                if args.len() != 3 {
                    println!("Usage: compress <source_path> <output_path>");
                    continue;
                }
                println!("Starting compression...");
                let compressor = Compressor::new(args[1].to_string(), args[2].to_string());
                match compressor.compress() {
                    Ok(_) => println!("Compression completed successfully!"),
                    Err(e) => println!("Error during compression: {}", e),
                }
            }
            "decompress" => {
                if args.len() != 3 {
                    println!("Usage: decompress <archive_path> <output_dir>");
                    continue;
                }
                println!("Starting decompression...");
                let compressor = Compressor::new(args[1].to_string(), args[2].to_string());
                match compressor.decompress() {
                    Ok(_) => println!("Decompression completed successfully!"),
                    Err(e) => println!("Error during decompression: {}", e),
                }
            }
            _ => {
                println!("Unknown command. Type 'help' for available commands.");
            }
        }
    }

    Ok(())
}

fn main() -> io::Result<()> {
    run_shell()
}
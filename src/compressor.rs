use std::fs::{self, File};
use std::io::{self, Read, Write, BufReader, BufWriter, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use crate::models::{FileEntry, FileInfo};
use crate::utils::{format_size, format_time};

pub struct Compressor {
    input_path: String,
    output_path: String,
}

impl Compressor {
    pub fn new(input_path: String, output_path: String) -> Self {
        Compressor {
            input_path,
            output_path,
        }
    }

    pub fn find_files(search_name: &str) -> io::Result<Vec<FileInfo>> {
        let mut matches = Vec::new();
        
        fn search_dir(dir: &Path, search_name: &str, matches: &mut Vec<FileInfo>) -> io::Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                let name = path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("")
                    .to_string();

                if name.to_lowercase().contains(&search_name.to_lowercase()) {
                    let metadata = entry.metadata()?;
                    matches.push(FileInfo {
                        name,
                        path: path.clone(),
                        size: if metadata.is_file() { metadata.len() } else { 0 },
                        last_modified: format_time(metadata.modified()?),
                        is_dir: metadata.is_dir(),
                    });
                }
                    
                if path.is_dir() {
                    search_dir(&path, search_name, matches)?;
                }
            }
            Ok(())
        }

        search_dir(Path::new("."), search_name, &mut matches)?;
        Ok(matches)
    }

    pub fn get_files_info(path: &str) -> io::Result<Vec<FileInfo>> {
        let path = Path::new(path);
        let mut files = Vec::new();

        if path.is_file() {
            let metadata = fs::metadata(path)?;
            files.push(FileInfo {
                name: path.file_name().unwrap_or_default().to_string_lossy().into_owned(),
                path: path.to_path_buf(),
                size: metadata.len(),
                last_modified: format_time(metadata.modified()?),
                is_dir: false,
            });
        } else if path.is_dir() {
            for entry in fs::read_dir(path)? {
                let entry = entry?;
                let metadata = entry.metadata()?;
                files.push(FileInfo {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path(),
                    size: if metadata.is_file() { metadata.len() } else { 0 },
                    last_modified: format_time(metadata.modified()?),
                    is_dir: metadata.is_dir(),
                });
            }
        }

        Ok(files)
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
                    writer.write_all(&[byte])?;
                    writer.write_all(&count.to_le_bytes())?;
                    bytes_written += 5;
                    
                    current_byte = Some(buffer[0]);
                    count = 1;
                }
            }
        }

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

    pub fn compress(&self) -> io::Result<()> {
        match Self::handle_existing_file(&self.output_path) {
            Ok(true) => {},
            Ok(false) => {}, 
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                let new_path = e.to_string();
                return Compressor::new(self.input_path.clone(), new_path).compress();
            }
            Err(e) => return Err(e),
        }

        let input_path = Path::new(&self.input_path);
        let output_file = File::create(&self.output_path)?;
        let mut writer = BufWriter::new(output_file);

        let files = Self::collect_files(input_path)?;
        
        writer.write_all(&(files.len() as u64).to_le_bytes())?;
        
        let header_pos = (files.len() as u64) * 
            ((std::mem::size_of::<u64>() as u64 * 2) +
             (std::mem::size_of::<u32>() as u64)); 
        let mut current_offset = header_pos;
        
        let mut file_entries = Vec::new();

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

        writer.seek(SeekFrom::Start(8))?; 
        for entry in file_entries {
            let path_bytes = entry.path.as_bytes();
            writer.write_all(&(path_bytes.len() as u32).to_le_bytes())?;
            writer.write_all(path_bytes)?;
            writer.write_all(&entry.size.to_le_bytes())?;
            writer.write_all(&entry.offset.to_le_bytes())?;
        }

        writer.flush()?;
        Ok(())
    }

    pub fn decompress(&self) -> io::Result<()> {
        let input_file = File::open(&self.input_path)?;
        let mut reader = BufReader::new(input_file);
        
        let mut count_buffer = [0; 8];
        reader.read_exact(&mut count_buffer)?;
        let num_files = u64::from_le_bytes(count_buffer);
        
        let mut files = Vec::new();
        for _ in 0..num_files {
            let mut len_buffer = [0; 4];
            reader.read_exact(&mut len_buffer)?;
            let path_len = u32::from_le_bytes(len_buffer) as usize;
            
            let mut path_buffer = vec![0; path_len];
            reader.read_exact(&mut path_buffer)?;
            let path = String::from_utf8(path_buffer).unwrap();
            
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

        let output_dir = Path::new(&self.output_path);
        fs::create_dir_all(output_dir)?;

        for file_entry in files {
            let output_path = output_dir.join(&file_entry.path);
            println!("Extracting: {}", output_path.display());
            
            if let Some(parent) = output_path.parent() {
                fs::create_dir_all(parent)?;
            }
            
            let output_file = File::create(output_path)?;
            let mut writer = BufWriter::new(output_file);

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
                
                bytes_read += 5;
            }
            writer.flush()?;
        }

        Ok(())
    }

    pub fn display_files(files: &[FileInfo]) {
        println!("\nAvailable files/folders:");
        println!("{:<5} {:<30} {:<15} {:<20} {}", "No.", "Name", "Size", "Last Modified", "Type");
        println!("{:-<80}", "");

        for (i, file) in files.iter().enumerate() {
            let size_str = if file.is_dir {
                "-".to_string()
            } else {
                format_size(file.size)
            };
            println!(
                "{:<5} {:<30} {:<15} {:<20} {}",
                i + 1,
                if file.name.len() > 30 {
                    format!("{}...", &file.name[..27])
                } else {
                    file.name.clone()
                },
                size_str,
                file.last_modified,
                if file.is_dir { "Directory" } else { "File" }
            );
        }
    }
}

use std::io::{self, Write};
use crate::compressor::Compressor;

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

pub fn run_shell() -> io::Result<()> {
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
                if args.len() != 2 {
                    println!("Usage: compress <filename/foldername>");
                    continue;
                }
                
                let matches = match Compressor::find_files(args[1]) {
                    Ok(files) => files,
                    Err(e) => {
                        println!("Error searching for files: {}", e);
                        continue;
                    }
                };

                if matches.is_empty() {
                    println!("No files or folders found matching '{}'", args[1]);
                    continue;
                }

                println!("\nFound {} matches:", matches.len());
                Compressor::display_files(&matches);

                println!("\nEnter the number of the file/folder to compress (1-{}):", matches.len());
                let mut selection = String::new();
                io::stdin().read_line(&mut selection)?;
                
                let index = match selection.trim().parse::<usize>() {
                    Ok(n) if n > 0 && n <= matches.len() => n - 1,
                    _ => {
                        println!("Invalid selection.");
                        continue;
                    }
                };

                let selected = &matches[index];
                
                let default_output = format!("{}.compressed", selected.name);
                println!("\nChoose output option:");
                println!("1. Save as {}", default_output);
                println!("2. Specify different name");
                
                let mut choice = String::new();
                io::stdin().read_line(&mut choice)?;
                
                let output_path = match choice.trim() {
                    "1" => default_output,
                    "2" => {
                        println!("Enter output filename:");
                        let mut custom_name = String::new();
                        io::stdin().read_line(&mut custom_name)?;
                        custom_name.trim().to_string()
                    }
                    _ => {
                        println!("Invalid choice. Using default name.");
                        default_output
                    }
                };

                println!("Starting compression...");
                let compressor = Compressor::new(selected.path.to_string_lossy().to_string(), output_path);
                match compressor.compress() {
                    Ok(_) => println!("Compression completed successfully!"),
                    Err(e) => println!("Error during compression: {}", e),
                }
            }
            "decompress" => {
                if args.len() != 2 {
                    println!("Usage: decompress <archive_name>");
                    continue;
                }

                let matches = match Compressor::find_files(args[1]) {
                    Ok(files) => files.into_iter()
                        .filter(|f| f.name.ends_with(".compressed"))
                        .collect::<Vec<_>>(),
                    Err(e) => {
                        println!("Error searching for archives: {}", e);
                        continue;
                    }
                };

                if matches.is_empty() {
                    println!("No compressed archives found matching '{}'", args[1]);
                    continue;
                }

                println!("\nFound {} compressed archives:", matches.len());
                Compressor::display_files(&matches);

                println!("\nEnter the number of the archive to decompress (1-{}):", matches.len());
                let mut selection = String::new();
                io::stdin().read_line(&mut selection)?;
                
                let index = match selection.trim().parse::<usize>() {
                    Ok(n) if n > 0 && n <= matches.len() => n - 1,
                    _ => {
                        println!("Invalid selection.");
                        continue;
                    }
                };

                let selected = &matches[index];

                println!("Enter extraction directory (press Enter for current directory):");
                let mut extract_dir = String::new();
                io::stdin().read_line(&mut extract_dir)?;
                let extract_dir = extract_dir.trim();
                let output_dir = if extract_dir.is_empty() { "." } else { extract_dir };

                println!("Starting decompression...");
                let compressor = Compressor::new(selected.path.to_string_lossy().to_string(), output_dir.to_string());
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
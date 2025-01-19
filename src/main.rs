use std::io;

mod models;
mod shell;
mod compressor;
mod utils;


fn main() -> io::Result<()> {
    shell::run_shell()
}
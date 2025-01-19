mod models;
mod shell;
mod compressor;
mod utils;

use std::io;


fn main() -> io::Result<()> {
    shell::run_shell()
}
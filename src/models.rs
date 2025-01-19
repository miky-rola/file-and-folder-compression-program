use std::path::PathBuf;

#[derive(Debug)]
pub struct FileEntry {
    pub path: String,
    pub size: u64,
    pub offset: u64,
}

#[derive(Debug)]
pub struct FileInfo {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    pub last_modified: String,
    pub is_dir: bool,
}

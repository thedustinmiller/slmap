use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Abstraction over filesystem operations for testability
pub trait FileSystem {
    /// Check if a path exists
    fn exists(&self, path: &Path) -> bool;

    /// Check if a path is a symlink
    fn is_symlink(&self, path: &Path) -> bool;

    /// Read the target of a symlink
    fn read_link(&self, path: &Path) -> io::Result<PathBuf>;

    /// Create a symbolic link
    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()>;

    /// Remove a file or symlink
    fn remove_file(&self, path: &Path) -> io::Result<()>;

    /// Create all parent directories
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;

    /// Check if path is a directory
    fn is_dir(&self, path: &Path) -> bool;

    /// Rename/move a file
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;
}

/// Real filesystem implementation
#[derive(Debug, Default)]
pub struct RealFileSystem;

impl FileSystem for RealFileSystem {
    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn is_symlink(&self, path: &Path) -> bool {
        path.is_symlink()
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        fs::read_link(path)
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(target, link)
        }

        #[cfg(windows)]
        {
            if target.is_dir() {
                std::os::windows::fs::symlink_dir(target, link)
            } else {
                std::os::windows::fs::symlink_file(target, link)
            }
        }
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        fs::create_dir_all(path)
    }

    fn is_dir(&self, path: &Path) -> bool {
        path.is_dir()
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }
}

/// In-memory filesystem for testing
#[derive(Debug, Default, Clone)]
pub struct MemoryFileSystem {
    files: HashMap<PathBuf, FileEntry>,
}

#[derive(Debug, Clone)]
enum FileEntry {
    File,
    Directory,
    Symlink(PathBuf),
}

impl MemoryFileSystem {
    pub fn new() -> Self {
        Self {
            files: HashMap::new(),
        }
    }

    /// Add a regular file to the filesystem
    pub fn add_file(&mut self, path: PathBuf) {
        self.files.insert(path, FileEntry::File);
    }

    /// Add a directory to the filesystem
    pub fn add_dir(&mut self, path: PathBuf) {
        self.files.insert(path, FileEntry::Directory);
    }

    /// Add a symlink to the filesystem
    pub fn add_symlink(&mut self, link: PathBuf, target: PathBuf) {
        self.files.insert(link, FileEntry::Symlink(target));
    }

    /// Check if filesystem contains a path
    pub fn contains(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    /// Get the target of a symlink (for testing)
    pub fn get_symlink_target(&self, link: &Path) -> Option<PathBuf> {
        match self.files.get(link) {
            Some(FileEntry::Symlink(target)) => Some(target.clone()),
            _ => None,
        }
    }
}

impl FileSystem for MemoryFileSystem {
    fn exists(&self, path: &Path) -> bool {
        self.files.contains_key(path)
    }

    fn is_symlink(&self, path: &Path) -> bool {
        matches!(self.files.get(path), Some(FileEntry::Symlink(_)))
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        match self.files.get(path) {
            Some(FileEntry::Symlink(target)) => Ok(target.clone()),
            _ => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "not a symlink",
            )),
        }
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        if self.exists(link) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "link already exists",
            ));
        }
        // Note: In real implementation, we'd need interior mutability
        // This is a simplified version for demonstration
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        if !self.exists(path) {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "file not found",
            ));
        }
        Ok(())
    }

    fn create_dir_all(&self, _path: &Path) -> io::Result<()> {
        Ok(())
    }

    fn is_dir(&self, path: &Path) -> bool {
        matches!(self.files.get(path), Some(FileEntry::Directory))
    }

    fn rename(&self, _from: &Path, _to: &Path) -> io::Result<()> {
        Ok(())
    }
}

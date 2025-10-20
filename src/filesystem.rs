use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

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
    files: Arc<Mutex<HashMap<PathBuf, FileEntry>>>,
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
            files: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Add a regular file to the filesystem
    pub fn add_file(&self, path: PathBuf) {
        let mut files = self.files.lock().unwrap();
        files.insert(path, FileEntry::File);
    }

    /// Add a directory to the filesystem
    pub fn add_dir(&self, path: PathBuf) {
        let mut files = self.files.lock().unwrap();
        files.insert(path, FileEntry::Directory);
    }

    /// Add a symlink to the filesystem
    pub fn add_symlink(&self, link: PathBuf, target: PathBuf) {
        let mut files = self.files.lock().unwrap();
        files.insert(link, FileEntry::Symlink(target));
    }

    /// Check if filesystem contains a path
    pub fn contains(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    /// Get the target of a symlink (for testing)
    pub fn get_symlink_target(&self, link: &Path) -> Option<PathBuf> {
        let files = self.files.lock().unwrap();
        match files.get(link) {
            Some(FileEntry::Symlink(target)) => Some(target.clone()),
            _ => None,
        }
    }

    /// Get number of entries (for testing)
    pub fn len(&self) -> usize {
        let files = self.files.lock().unwrap();
        files.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        let files = self.files.lock().unwrap();
        files.is_empty()
    }

    /// Clear all entries
    pub fn clear(&self) {
        let mut files = self.files.lock().unwrap();
        files.clear();
    }
}

impl FileSystem for MemoryFileSystem {
    fn exists(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        files.contains_key(path)
    }

    fn is_symlink(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        matches!(files.get(path), Some(FileEntry::Symlink(_)))
    }

    fn read_link(&self, path: &Path) -> io::Result<PathBuf> {
        let files = self.files.lock().unwrap();
        match files.get(path) {
            Some(FileEntry::Symlink(target)) => Ok(target.clone()),
            _ => Err(io::Error::new(io::ErrorKind::InvalidInput, "not a symlink")),
        }
    }

    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        if files.contains_key(link) {
            return Err(io::Error::new(
                io::ErrorKind::AlreadyExists,
                "link already exists",
            ));
        }
        files.insert(link.to_path_buf(), FileEntry::Symlink(target.to_path_buf()));
        Ok(())
    }

    fn remove_file(&self, path: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        if !files.contains_key(path) {
            return Err(io::Error::new(io::ErrorKind::NotFound, "file not found"));
        }
        files.remove(path);
        Ok(())
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        files.insert(path.to_path_buf(), FileEntry::Directory);
        Ok(())
    }

    fn is_dir(&self, path: &Path) -> bool {
        let files = self.files.lock().unwrap();
        matches!(files.get(path), Some(FileEntry::Directory))
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        let mut files = self.files.lock().unwrap();
        if let Some(entry) = files.remove(from) {
            files.insert(to.to_path_buf(), entry);
            Ok(())
        } else {
            Err(io::Error::new(
                io::ErrorKind::NotFound,
                "source file not found",
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_memory_fs_basic_operations() {
        let fs = MemoryFileSystem::new();
        assert!(fs.is_empty());

        // Add a file
        fs.add_file(PathBuf::from("/test.txt"));
        assert!(fs.exists(&PathBuf::from("/test.txt")));
        assert_eq!(fs.len(), 1);
        assert!(!fs.is_symlink(&PathBuf::from("/test.txt")));
    }

    #[test]
    fn test_memory_fs_symlink() {
        let fs = MemoryFileSystem::new();

        let target = PathBuf::from("/target");
        let link = PathBuf::from("/link");

        // Create symlink
        fs.symlink(&target, &link).unwrap();

        assert!(fs.exists(&link));
        assert!(fs.is_symlink(&link));
        assert_eq!(fs.read_link(&link).unwrap(), target);
    }

    #[test]
    fn test_memory_fs_symlink_already_exists() {
        let fs = MemoryFileSystem::new();

        let target = PathBuf::from("/target");
        let link = PathBuf::from("/link");

        fs.symlink(&target, &link).unwrap();

        // Try to create again
        let result = fs.symlink(&target, &link);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::AlreadyExists);
    }

    #[test]
    fn test_memory_fs_remove_file() {
        let fs = MemoryFileSystem::new();

        let link = PathBuf::from("/link");
        fs.add_file(link.clone());

        assert!(fs.exists(&link));

        fs.remove_file(&link).unwrap();
        assert!(!fs.exists(&link));
    }

    #[test]
    fn test_memory_fs_remove_nonexistent() {
        let fs = MemoryFileSystem::new();

        let result = fs.remove_file(&PathBuf::from("/nonexistent"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }

    #[test]
    fn test_memory_fs_directory() {
        let fs = MemoryFileSystem::new();

        let dir = PathBuf::from("/dir");
        fs.add_dir(dir.clone());

        assert!(fs.exists(&dir));
        assert!(fs.is_dir(&dir));
        assert!(!fs.is_symlink(&dir));
    }

    #[test]
    fn test_memory_fs_create_dir_all() {
        let fs = MemoryFileSystem::new();

        let dir = PathBuf::from("/parent/child");
        fs.create_dir_all(&dir).unwrap();

        assert!(fs.exists(&dir));
        assert!(fs.is_dir(&dir));
    }

    #[test]
    fn test_memory_fs_rename() {
        let fs = MemoryFileSystem::new();

        let from = PathBuf::from("/from");
        let to = PathBuf::from("/to");

        fs.add_file(from.clone());
        assert!(fs.exists(&from));

        fs.rename(&from, &to).unwrap();

        assert!(!fs.exists(&from));
        assert!(fs.exists(&to));
    }

    #[test]
    fn test_memory_fs_rename_nonexistent() {
        let fs = MemoryFileSystem::new();

        let result = fs.rename(&PathBuf::from("/from"), &PathBuf::from("/to"));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::NotFound);
    }
}

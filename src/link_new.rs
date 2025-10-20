use crate::error::{Result, SlmapError};
use crate::filesystem::FileSystem;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Represents a symbolic link configuration
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct Link {
    pub target: String,
    pub link_name: String,
    #[serde(default = "default_directory")]
    pub directory: bool,
    #[serde(default = "default_root")]
    pub root: bool,
}

fn default_root() -> bool {
    false
}

fn default_directory() -> bool {
    false
}

/// Status of a link in the filesystem
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
    /// Link doesn't exist
    Missing,
    /// Link exists and points to correct target
    Correct,
    /// Link exists but points to wrong target
    Incorrect { actual: PathBuf, expected: PathBuf },
    /// Path exists but is not a symlink
    NotSymlink(PathBuf),
    /// Error checking link status
    Error(String),
}

impl Link {
    /// Resolve the target path with shell expansion
    pub fn resolved_target(&self) -> Result<PathBuf> {
        resolve_path(&self.target)
    }

    /// Resolve the link name path with shell expansion
    pub fn resolved_link_name(&self) -> Result<PathBuf> {
        resolve_path(&self.link_name)
    }

    /// Check the current status of this link in the filesystem
    pub fn check_status(&self, fs: &dyn FileSystem) -> LinkStatus {
        let target = match self.resolved_target() {
            Ok(t) => t,
            Err(e) => return LinkStatus::Error(e.to_string()),
        };

        let link_name = match self.resolved_link_name() {
            Ok(l) => l,
            Err(e) => return LinkStatus::Error(e.to_string()),
        };

        if !fs.exists(&link_name) {
            return LinkStatus::Missing;
        }

        if !fs.is_symlink(&link_name) {
            return LinkStatus::NotSymlink(link_name);
        }

        match fs.read_link(&link_name) {
            Ok(actual_target) => {
                if actual_target == target {
                    LinkStatus::Correct
                } else {
                    LinkStatus::Incorrect {
                        actual: actual_target,
                        expected: target,
                    }
                }
            }
            Err(e) => LinkStatus::Error(e.to_string()),
        }
    }

    /// Validate that this link configuration is valid
    pub fn validate(&self, fs: &dyn FileSystem, check_target_exists: bool) -> Result<()> {
        // Resolve paths
        let target = self.resolved_target()?;
        let link_name = self.resolved_link_name()?;

        // Check for path traversal
        if is_suspicious_path(&link_name) {
            return Err(SlmapError::UnsafePath(link_name));
        }

        // Check if target exists (optional)
        if check_target_exists && !fs.exists(&target) {
            return Err(SlmapError::TargetNotFound(target));
        }

        Ok(())
    }

    /// Create this symlink (non-transactional)
    pub fn create(&self, fs: &dyn FileSystem) -> Result<()> {
        let target = self.resolved_target()?;
        let link_name = self.resolved_link_name()?;

        // Validate first
        self.validate(fs, true)?;

        // Check if link already exists
        if fs.exists(&link_name) {
            return Err(SlmapError::LinkExists(link_name));
        }

        // Create parent directories
        if let Some(parent) = link_name.parent() {
            fs.create_dir_all(parent)?;
        }

        // Create symlink
        fs.symlink(&target, &link_name)?;

        Ok(())
    }

    /// Remove this symlink (non-transactional)
    pub fn remove(&self, fs: &dyn FileSystem) -> Result<()> {
        let link_name = self.resolved_link_name()?;

        if !fs.is_symlink(&link_name) {
            return Err(SlmapError::NotASymlink(link_name));
        }

        fs.remove_file(&link_name)?;
        Ok(())
    }
}

/// Resolve a path with shell expansion
fn resolve_path(path: &str) -> Result<PathBuf> {
    match shellexpand::full(path) {
        Ok(expanded) => Ok(PathBuf::from(expanded.into_owned())),
        Err(e) => Err(SlmapError::PathResolution {
            path: path.to_string(),
            var: e.var_name,
        }),
    }
}

/// Check if a path looks suspicious (basic path traversal detection)
fn is_suspicious_path(path: &PathBuf) -> bool {
    // This is a basic check - could be more sophisticated
    let path_str = path.to_string_lossy();
    path_str.contains("..") && !path_str.starts_with('.')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFileSystem;

    #[test]
    fn test_link_resolved_paths() {
        let link = Link {
            target: "/home/user/target".to_string(),
            link_name: "/home/user/link".to_string(),
            directory: false,
            root: false,
        };

        assert_eq!(
            link.resolved_target().unwrap(),
            PathBuf::from("/home/user/target")
        );
        assert_eq!(
            link.resolved_link_name().unwrap(),
            PathBuf::from("/home/user/link")
        );
    }

    #[test]
    fn test_link_status_missing() {
        let fs = MemoryFileSystem::new();
        let link = Link {
            target: "/target".to_string(),
            link_name: "/link".to_string(),
            directory: false,
            root: false,
        };

        assert_eq!(link.check_status(&fs), LinkStatus::Missing);
    }

    #[test]
    fn test_suspicious_path_detection() {
        assert!(!is_suspicious_path(&PathBuf::from("/home/user/.config")));
        assert!(!is_suspicious_path(&PathBuf::from("./local/path")));
        assert!(is_suspicious_path(&PathBuf::from("/home/../etc/passwd")));
    }
}

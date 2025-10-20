use std::io;
use std::path::PathBuf;
use thiserror::Error;

/// Errors that can occur during slmap operations
#[derive(Error, Debug)]
pub enum SlmapError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("Failed to parse TOML config: {0}")]
    TomlParse(#[from] toml::de::Error),

    #[error("Failed to resolve path '{path}': variable '{var}' not found")]
    PathResolution { path: String, var: String },

    #[error("Target does not exist: {0}")]
    TargetNotFound(PathBuf),

    #[error("Link already exists and is not a symlink: {0}")]
    NotASymlink(PathBuf),

    #[error("Link exists but points to wrong target: {link}\n  Expected: {expected}\n  Actual: {actual}")]
    IncorrectTarget {
        link: PathBuf,
        expected: PathBuf,
        actual: PathBuf,
    },

    #[error("Cannot create link, file exists at: {0}")]
    LinkExists(PathBuf),

    #[error("Transaction failed, rolled back {0} operations")]
    TransactionFailed(usize),

    #[error("Path traversal detected in: {0}")]
    UnsafePath(PathBuf),

    #[error("Permission denied: {0}")]
    PermissionDenied(PathBuf),

    #[error("Link validation failed: {0}")]
    ValidationFailed(String),
}

pub type Result<T> = std::result::Result<T, SlmapError>;

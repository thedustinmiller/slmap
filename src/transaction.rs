use crate::error::{Result, SlmapError};
use crate::filesystem::FileSystem;
use std::path::PathBuf;

/// Represents an operation that can be rolled back
#[derive(Debug, Clone)]
pub enum Operation {
    CreateLink {
        target: PathBuf,
        link: PathBuf,
    },
    RemoveLink {
        link: PathBuf,
    },
    UpdateLink {
        link: PathBuf,
        old_target: PathBuf,
        new_target: PathBuf,
    },
}

/// Rollback action for an operation
#[derive(Debug, Clone)]
pub enum Rollback {
    RemoveLink { link: PathBuf },
    CreateLink { target: PathBuf, link: PathBuf },
    RestoreLink { link: PathBuf, target: PathBuf },
}

impl Operation {
    /// Execute the operation and return a rollback action
    pub fn execute(&self, fs: &dyn FileSystem) -> Result<Rollback> {
        match self {
            Operation::CreateLink { target, link } => {
                // Create parent directories if needed
                if let Some(parent) = link.parent() {
                    fs.create_dir_all(parent)?;
                }

                // Create the symlink
                fs.symlink(target, link)?;

                Ok(Rollback::RemoveLink { link: link.clone() })
            }

            Operation::RemoveLink { link } => {
                // Read the current target before removing (for rollback)
                let target = fs.read_link(link)?;

                // Remove the symlink
                fs.remove_file(link)?;

                Ok(Rollback::CreateLink {
                    target,
                    link: link.clone(),
                })
            }

            Operation::UpdateLink {
                link,
                old_target,
                new_target,
            } => {
                // Remove old link
                fs.remove_file(link)?;

                // Create new link
                fs.symlink(new_target, link)?;

                Ok(Rollback::RestoreLink {
                    link: link.clone(),
                    target: old_target.clone(),
                })
            }
        }
    }
}

impl Rollback {
    /// Execute the rollback action
    pub fn execute(&self, fs: &dyn FileSystem) -> Result<()> {
        match self {
            Rollback::RemoveLink { link } => {
                fs.remove_file(link)?;
                Ok(())
            }

            Rollback::CreateLink { target, link } => {
                if let Some(parent) = link.parent() {
                    fs.create_dir_all(parent)?;
                }
                fs.symlink(target, link)?;
                Ok(())
            }

            Rollback::RestoreLink { link, target } => {
                // Remove current link if it exists
                if fs.exists(link) {
                    fs.remove_file(link)?;
                }
                // Restore old link
                fs.symlink(target, link)?;
                Ok(())
            }
        }
    }
}

/// Transaction manages atomic execution of multiple operations
pub struct Transaction {
    operations: Vec<Operation>,
    completed: Vec<Rollback>,
}

impl Transaction {
    /// Create a new empty transaction
    pub fn new() -> Self {
        Self {
            operations: Vec::new(),
            completed: Vec::new(),
        }
    }

    /// Add an operation to the transaction
    pub fn add_operation(&mut self, operation: Operation) {
        self.operations.push(operation);
    }

    /// Execute all operations atomically
    /// If any operation fails, all completed operations are rolled back
    pub fn execute(&mut self, fs: &dyn FileSystem) -> Result<()> {
        for operation in &self.operations {
            match operation.execute(fs) {
                Ok(rollback) => {
                    self.completed.push(rollback);
                }
                Err(e) => {
                    // Operation failed, rollback all completed operations
                    self.rollback(fs);
                    return Err(SlmapError::TransactionFailed(self.completed.len()));
                }
            }
        }
        Ok(())
    }

    /// Rollback all completed operations in reverse order
    fn rollback(&mut self, fs: &dyn FileSystem) {
        for rollback in self.completed.drain(..).rev() {
            // Best effort rollback - ignore errors
            let _ = rollback.execute(fs);
        }
    }

    /// Get the number of operations in this transaction
    pub fn len(&self) -> usize {
        self.operations.len()
    }

    /// Check if transaction is empty
    pub fn is_empty(&self) -> bool {
        self.operations.is_empty()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFileSystem;

    #[test]
    fn test_transaction_empty() {
        let transaction = Transaction::new();
        assert!(transaction.is_empty());
        assert_eq!(transaction.len(), 0);
    }

    #[test]
    fn test_transaction_add_operations() {
        let mut transaction = Transaction::new();
        transaction.add_operation(Operation::CreateLink {
            target: PathBuf::from("/target"),
            link: PathBuf::from("/link"),
        });
        assert_eq!(transaction.len(), 1);
        assert!(!transaction.is_empty());
    }
}

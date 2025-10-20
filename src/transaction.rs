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
                Err(_e) => {
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

    #[test]
    fn test_transaction_single_create_link() {
        let fs = MemoryFileSystem::new();
        let mut transaction = Transaction::new();

        let target = PathBuf::from("/target");
        let link = PathBuf::from("/link");

        transaction.add_operation(Operation::CreateLink {
            target: target.clone(),
            link: link.clone(),
        });

        // Execute transaction
        let result = transaction.execute(&fs);
        assert!(result.is_ok());

        // Verify link was created
        assert!(fs.exists(&link));
        assert!(fs.is_symlink(&link));
        assert_eq!(fs.read_link(&link).unwrap(), target);
    }

    #[test]
    fn test_transaction_multiple_create_links() {
        let fs = MemoryFileSystem::new();
        let mut transaction = Transaction::new();

        let target1 = PathBuf::from("/target1");
        let link1 = PathBuf::from("/link1");
        let target2 = PathBuf::from("/target2");
        let link2 = PathBuf::from("/link2");

        transaction.add_operation(Operation::CreateLink {
            target: target1.clone(),
            link: link1.clone(),
        });
        transaction.add_operation(Operation::CreateLink {
            target: target2.clone(),
            link: link2.clone(),
        });

        // Execute transaction
        let result = transaction.execute(&fs);
        assert!(result.is_ok());

        // Verify both links were created
        assert!(fs.exists(&link1));
        assert!(fs.exists(&link2));
        assert_eq!(fs.read_link(&link1).unwrap(), target1);
        assert_eq!(fs.read_link(&link2).unwrap(), target2);
    }

    #[test]
    fn test_transaction_rollback_on_failure() {
        let fs = MemoryFileSystem::new();
        let mut transaction = Transaction::new();

        let target1 = PathBuf::from("/target1");
        let link1 = PathBuf::from("/link1");
        let target2 = PathBuf::from("/target2");
        let link2 = PathBuf::from("/link2");

        // Pre-create link2 to cause a conflict
        fs.add_file(link2.clone());

        transaction.add_operation(Operation::CreateLink {
            target: target1.clone(),
            link: link1.clone(),
        });
        transaction.add_operation(Operation::CreateLink {
            target: target2.clone(),
            link: link2.clone(), // This will fail
        });

        // Execute transaction
        let result = transaction.execute(&fs);
        assert!(result.is_err());

        // Verify link1 was rolled back (should not exist as symlink)
        assert!(!fs.is_symlink(&link1), "link1 should have been rolled back");

        // Verify link2 still exists as regular file
        assert!(fs.exists(&link2));
        assert!(!fs.is_symlink(&link2));
    }

    #[test]
    fn test_transaction_remove_link() {
        let fs = MemoryFileSystem::new();

        let target = PathBuf::from("/target");
        let link = PathBuf::from("/link");

        // Create a symlink first
        fs.add_symlink(link.clone(), target.clone());
        assert!(fs.exists(&link));

        // Create transaction to remove it
        let mut transaction = Transaction::new();
        transaction.add_operation(Operation::RemoveLink { link: link.clone() });

        // Execute
        let result = transaction.execute(&fs);
        assert!(result.is_ok());

        // Verify link was removed
        assert!(!fs.exists(&link));
    }

    #[test]
    fn test_transaction_remove_rollback() {
        let fs = MemoryFileSystem::new();

        let target1 = PathBuf::from("/target1");
        let link1 = PathBuf::from("/link1");
        let link2 = PathBuf::from("/link2");

        // Create link1
        fs.add_symlink(link1.clone(), target1.clone());

        // Create conflict at link2
        fs.add_file(link2.clone());

        // Transaction: remove link1, then try to create link2 (will fail)
        let mut transaction = Transaction::new();
        transaction.add_operation(Operation::RemoveLink {
            link: link1.clone(),
        });
        transaction.add_operation(Operation::CreateLink {
            target: PathBuf::from("/target2"),
            link: link2.clone(),
        });

        // Execute - should fail and rollback
        let result = transaction.execute(&fs);
        assert!(result.is_err());

        // Verify link1 was restored
        assert!(fs.exists(&link1), "link1 should be restored");
        assert!(fs.is_symlink(&link1), "link1 should be a symlink");
        assert_eq!(
            fs.read_link(&link1).unwrap(),
            target1,
            "link1 should point to original target"
        );
    }

    #[test]
    fn test_transaction_update_link() {
        let fs = MemoryFileSystem::new();

        let old_target = PathBuf::from("/old_target");
        let new_target = PathBuf::from("/new_target");
        let link = PathBuf::from("/link");

        // Create link with old target
        fs.add_symlink(link.clone(), old_target.clone());

        // Create transaction to update it
        let mut transaction = Transaction::new();
        transaction.add_operation(Operation::UpdateLink {
            link: link.clone(),
            old_target: old_target.clone(),
            new_target: new_target.clone(),
        });

        // Execute
        let result = transaction.execute(&fs);
        assert!(result.is_ok());

        // Verify link now points to new target
        assert!(fs.exists(&link));
        assert!(fs.is_symlink(&link));
        assert_eq!(fs.read_link(&link).unwrap(), new_target);
    }

    #[test]
    fn test_transaction_complex_rollback() {
        let fs = MemoryFileSystem::new();

        let target1 = PathBuf::from("/target1");
        let link1 = PathBuf::from("/link1");
        let target2 = PathBuf::from("/target2");
        let link2 = PathBuf::from("/link2");
        let target3 = PathBuf::from("/target3");
        let link3 = PathBuf::from("/link3");

        // Pre-create link3 to cause failure
        fs.add_file(link3.clone());

        // Transaction with 3 operations, last one will fail
        let mut transaction = Transaction::new();
        transaction.add_operation(Operation::CreateLink {
            target: target1.clone(),
            link: link1.clone(),
        });
        transaction.add_operation(Operation::CreateLink {
            target: target2.clone(),
            link: link2.clone(),
        });
        transaction.add_operation(Operation::CreateLink {
            target: target3.clone(),
            link: link3.clone(), // Will fail
        });

        // Execute
        let result = transaction.execute(&fs);
        assert!(result.is_err());

        // Verify all successful operations were rolled back
        assert!(!fs.is_symlink(&link1), "link1 should be rolled back");
        assert!(!fs.is_symlink(&link2), "link2 should be rolled back");

        // Verify link3 is unchanged
        assert!(fs.exists(&link3));
        assert!(!fs.is_symlink(&link3));
    }

    #[test]
    fn test_operation_create_with_parent_dirs() {
        let fs = MemoryFileSystem::new();
        let mut transaction = Transaction::new();

        let target = PathBuf::from("/target");
        let link = PathBuf::from("/parent/child/link");

        transaction.add_operation(Operation::CreateLink {
            target: target.clone(),
            link: link.clone(),
        });

        // Execute
        let result = transaction.execute(&fs);
        assert!(result.is_ok());

        // Verify parent directory was created
        assert!(fs.exists(&PathBuf::from("/parent/child")));
        assert!(fs.is_dir(&PathBuf::from("/parent/child")));

        // Verify link was created
        assert!(fs.exists(&link));
        assert!(fs.is_symlink(&link));
    }

    #[test]
    fn test_transaction_default() {
        let transaction = Transaction::default();
        assert!(transaction.is_empty());
    }
}

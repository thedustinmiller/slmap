use crate::error::Result;
use crate::filesystem::FileSystem;
use crate::link_new::{Link, LinkStatus};
use crate::transaction::{Operation, Transaction};
use std::collections::HashMap;

/// Represents the desired state from configuration
pub type LinkMap = HashMap<String, Link>;

/// Represents a change that needs to be applied
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Change {
    Create { name: String, link: Link },
    Update { name: String, link: Link },
    Remove { name: String, link: Link },
    NoChange { name: String, link: Link },
}

/// Execution plan containing all changes to apply
#[derive(Debug, Default)]
pub struct ExecutionPlan {
    changes: Vec<Change>,
}

impl ExecutionPlan {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    pub fn add_change(&mut self, change: Change) {
        self.changes.push(change);
    }

    pub fn changes(&self) -> &[Change] {
        &self.changes
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.changes.len()
    }

    /// Get count of each change type
    pub fn summary(&self) -> (usize, usize, usize, usize) {
        let mut creates = 0;
        let mut updates = 0;
        let mut removes = 0;
        let mut no_changes = 0;

        for change in &self.changes {
            match change {
                Change::Create { .. } => creates += 1,
                Change::Update { .. } => updates += 1,
                Change::Remove { .. } => removes += 1,
                Change::NoChange { .. } => no_changes += 1,
            }
        }

        (creates, updates, removes, no_changes)
    }

    /// Convert plan into a transaction
    pub fn into_transaction(self) -> Result<Transaction> {
        let mut transaction = Transaction::new();

        for change in self.changes {
            match change {
                Change::Create { link, .. } => {
                    let target = link.resolved_target()?;
                    let link_name = link.resolved_link_name()?;
                    transaction.add_operation(Operation::CreateLink {
                        target,
                        link: link_name,
                    });
                }
                Change::Update { link, .. } => {
                    let target = link.resolved_target()?;
                    let link_name = link.resolved_link_name()?;
                    // For update, we need to read the old target first
                    // This is a simplified version - in practice, we'd pass the old target
                    transaction.add_operation(Operation::RemoveLink {
                        link: link_name.clone(),
                    });
                    transaction.add_operation(Operation::CreateLink {
                        target,
                        link: link_name,
                    });
                }
                Change::Remove { link, .. } => {
                    let link_name = link.resolved_link_name()?;
                    transaction.add_operation(Operation::RemoveLink { link: link_name });
                }
                Change::NoChange { .. } => {
                    // No operation needed
                }
            }
        }

        Ok(transaction)
    }
}

/// StateManager computes differences between desired and actual state
pub struct StateManager;

impl StateManager {
    /// Compute execution plan by comparing desired state with actual filesystem state
    ///
    /// This is the core of the declarative approach:
    /// - Inspect current state
    /// - Compare with desired state
    /// - Compute minimal set of changes needed
    pub fn compute_plan(desired: &LinkMap, fs: &dyn FileSystem) -> ExecutionPlan {
        let mut plan = ExecutionPlan::new();

        for (name, link) in desired {
            let status = link.check_status(fs);

            let change = match status {
                LinkStatus::Missing => Change::Create {
                    name: name.clone(),
                    link: link.clone(),
                },
                LinkStatus::Correct => Change::NoChange {
                    name: name.clone(),
                    link: link.clone(),
                },
                LinkStatus::Incorrect { .. } => Change::Update {
                    name: name.clone(),
                    link: link.clone(),
                },
                LinkStatus::NotSymlink(_) => {
                    // This is a conflict - for now, treat as needing update
                    // In a real implementation, we might want to handle this differently
                    Change::Update {
                        name: name.clone(),
                        link: link.clone(),
                    }
                }
                LinkStatus::Error(_) => {
                    // Skip entries with errors
                    continue;
                }
            };

            plan.add_change(change);
        }

        plan
    }

    /// Apply the desired state to the filesystem atomically
    ///
    /// This is idempotent: running it multiple times produces the same result
    /// This is atomic: either all changes succeed, or none are applied (rollback)
    pub fn apply(desired: &LinkMap, fs: &dyn FileSystem) -> Result<ExecutionPlan> {
        // Compute what needs to change
        let plan = Self::compute_plan(desired, fs);

        // Convert plan to transaction
        let mut transaction = plan.into_transaction()?;

        // Execute transaction (atomic - rolls back on failure)
        transaction.execute(fs)?;

        // Return the plan that was executed
        Ok(plan)
    }

    /// Validate all links in the desired state
    pub fn validate(desired: &LinkMap, fs: &dyn FileSystem) -> Result<Vec<String>> {
        let mut errors = Vec::new();

        for (name, link) in desired {
            if let Err(e) = link.validate(fs, false) {
                errors.push(format!("{}: {}", name, e));
            }
        }

        Ok(errors)
    }

    /// Check if the filesystem matches the desired state (idempotent check)
    pub fn is_synced(desired: &LinkMap, fs: &dyn FileSystem) -> bool {
        let plan = Self::compute_plan(desired, fs);
        let (creates, updates, removes, _) = plan.summary();
        creates == 0 && updates == 0 && removes == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFileSystem;

    #[test]
    fn test_empty_plan() {
        let plan = ExecutionPlan::new();
        assert!(plan.is_empty());
        assert_eq!(plan.len(), 0);
    }

    #[test]
    fn test_plan_summary() {
        let mut plan = ExecutionPlan::new();

        plan.add_change(Change::Create {
            name: "link1".to_string(),
            link: Link {
                target: "/target1".to_string(),
                link_name: "/link1".to_string(),
                directory: false,
                root: false,
            },
        });

        plan.add_change(Change::NoChange {
            name: "link2".to_string(),
            link: Link {
                target: "/target2".to_string(),
                link_name: "/link2".to_string(),
                directory: false,
                root: false,
            },
        });

        let (creates, updates, removes, no_changes) = plan.summary();
        assert_eq!(creates, 1);
        assert_eq!(updates, 0);
        assert_eq!(removes, 0);
        assert_eq!(no_changes, 1);
    }

    #[test]
    fn test_compute_plan_empty() {
        let desired = LinkMap::new();
        let fs = MemoryFileSystem::new();

        let plan = StateManager::compute_plan(&desired, &fs);
        assert!(plan.is_empty());
    }

    #[test]
    fn test_is_synced_empty() {
        let desired = LinkMap::new();
        let fs = MemoryFileSystem::new();

        assert!(StateManager::is_synced(&desired, &fs));
    }
}

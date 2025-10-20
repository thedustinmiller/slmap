// Re-export core types and modules
pub mod error;
pub mod filesystem;
pub mod link_new;
pub mod state;
pub mod transaction;

// Old link module for backwards compatibility during migration
pub mod link;

pub use error::{Result, SlmapError};
pub use filesystem::{FileSystem, MemoryFileSystem, RealFileSystem};
pub use link_new::{Link, LinkStatus};
pub use state::{Change, ExecutionPlan, LinkMap, StateManager};
pub use transaction::{Operation, Rollback, Transaction};

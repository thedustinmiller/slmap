# slmap Architecture - Refactored Design

## Overview

This document describes the new architecture for slmap, implementing a **declarative**, **atomic**, and **idempotent** design.

## Core Principles

### 1. Declarative
The system treats configuration as desired state rather than imperative commands. Users declare what links should exist, and the system figures out what changes are needed.

### 2. Atomic
All operations are transactional. Either all changes succeed, or none are applied (with automatic rollback).

### 3. Idempotent
Running the same operation multiple times produces the same result. The system converges to the desired state regardless of how many times it's applied.

## Module Structure

```
src/
├── lib.rs              - Public API and module exports
├── error.rs            - Custom error types using thiserror
├── filesystem.rs       - FileSystem trait abstraction
├── link_new.rs         - Refactored Link struct with proper error handling
├── state.rs            - StateManager for declarative operations
├── transaction.rs      - Transaction system with rollback
├── link.rs             - (Legacy, for backward compatibility)
└── main.rs             - CLI entry point
```

## Key Components

### Error Handling (`error.rs`)

**Before:**
```rust
let map: HashMap<String, Link> = toml::from_str(&file_string).unwrap(); // Panics!
```

**After:**
```rust
pub enum SlmapError {
    Io(#[from] io::Error),
    TomlParse(#[from] toml::de::Error),
    PathResolution { path: String, var: String },
    TargetNotFound(PathBuf),
    // ... more variants
}

pub type Result<T> = std::result::Result<T, SlmapError>;
```

**Benefits:**
- No more panics - all errors are handled gracefully
- Rich error context with thiserror
- Proper error propagation with `?` operator

### FileSystem Abstraction (`filesystem.rs`)

**Purpose:** Enable testing and cross-platform support

```rust
pub trait FileSystem {
    fn exists(&self, path: &Path) -> bool;
    fn is_symlink(&self, path: &Path) -> bool;
    fn symlink(&self, target: &Path, link: &Path) -> io::Result<()>;
    // ... more methods
}

pub struct RealFileSystem;      // Production implementation
pub struct MemoryFileSystem;    // Testing implementation
```

**Benefits:**
- Testable without touching real filesystem
- Cross-platform support (Unix/Windows)
- Dependency injection for better architecture

### Transaction System (`transaction.rs`)

**Purpose:** Atomic operations with automatic rollback

```rust
pub struct Transaction {
    operations: Vec<Operation>,
    completed: Vec<Rollback>,
}

impl Transaction {
    pub fn execute(&mut self, fs: &dyn FileSystem) -> Result<()> {
        for operation in &self.operations {
            match operation.execute(fs) {
                Ok(rollback) => self.completed.push(rollback),
                Err(e) => {
                    self.rollback(fs);  // Undo all completed operations
                    return Err(SlmapError::TransactionFailed(...));
                }
            }
        }
        Ok(())
    }
}
```

**Example:**
```rust
let mut transaction = Transaction::new();
transaction.add_operation(Operation::CreateLink { target, link });
transaction.add_operation(Operation::CreateLink { target2, link2 });

// If link2 creation fails, link1 is automatically rolled back
transaction.execute(&fs)?;
```

**Benefits:**
- All-or-nothing semantics
- Automatic rollback on failure
- No partial state left on errors

### State Manager (`state.rs`)

**Purpose:** Declarative state management - compute what needs to change

```rust
pub struct StateManager;

impl StateManager {
    // Compute diff between desired and actual state
    pub fn compute_plan(desired: &LinkMap, fs: &dyn FileSystem) -> ExecutionPlan;

    // Apply desired state atomically
    pub fn apply(desired: &LinkMap, fs: &dyn FileSystem) -> Result<ExecutionPlan>;

    // Check if filesystem matches desired state
    pub fn is_synced(desired: &LinkMap, fs: &dyn FileSystem) -> bool;
}
```

**How it works:**

1. **Inspect current state** - Check what links exist and where they point
2. **Compare with desired state** - Determine what's missing, incorrect, or extra
3. **Compute minimal changes** - Create an execution plan
4. **Execute atomically** - Apply all changes in a transaction

**Example:**
```rust
let desired = LinkMap::new();
desired.insert("vimrc", Link { ... });
desired.insert("zshrc", Link { ... });

// Compute what needs to change
let plan = StateManager::compute_plan(&desired, &fs);
let (creates, updates, removes, no_changes) = plan.summary();

// Apply changes atomically
StateManager::apply(&desired, &fs)?;

// Running again does nothing (idempotent)
StateManager::apply(&desired, &fs)?;  // No changes made
assert!(StateManager::is_synced(&desired, &fs));
```

### Link Struct (`link_new.rs`)

**Before:**
```rust
pub fn check_link(&self) -> LinkStatus {
    let target = self.resolved_target().unwrap();  // Panic!
    let link_name = self.resolved_link_name().unwrap();  // Panic!
    // ...
}
```

**After:**
```rust
pub fn check_status(&self, fs: &dyn FileSystem) -> LinkStatus {
    let target = match self.resolved_target() {
        Ok(t) => t,
        Err(e) => return LinkStatus::Error(e.to_string()),
    };
    // ... proper error handling throughout
}

pub fn validate(&self, fs: &dyn FileSystem, check_target_exists: bool) -> Result<()> {
    // Validate paths
    // Check for path traversal
    // Optionally verify target exists
}
```

**Benefits:**
- No panics - returns errors properly
- Validation before operations
- Path traversal detection
- Dependency injection (FileSystem trait)

## Usage Examples

### Basic Usage (Declarative)

```rust
use slmap::{LinkMap, Link, StateManager, RealFileSystem};

// Define desired state
let mut desired = LinkMap::new();
desired.insert("vimrc", Link {
    target: "~/dotfiles/vimrc".to_string(),
    link_name: "~/.vimrc".to_string(),
    directory: false,
    root: false,
});

// Apply state (idempotent and atomic)
let fs = RealFileSystem;
StateManager::apply(&desired, &fs)?;

// Running again is safe and does nothing
StateManager::apply(&desired, &fs)?;
```

### Check Status Before Applying

```rust
// Compute plan without making changes
let plan = StateManager::compute_plan(&desired, &fs);
let (creates, updates, removes, no_changes) = plan.summary();

println!("Will create: {}, update: {}, remove: {}", creates, updates, removes);

// Apply if user confirms
if user_confirms() {
    StateManager::apply(&desired, &fs)?;
}
```

### Manual Transaction

```rust
use slmap::{Transaction, Operation};

let mut transaction = Transaction::new();

transaction.add_operation(Operation::CreateLink {
    target: PathBuf::from("/home/user/target1"),
    link: PathBuf::from("/home/user/link1"),
});

transaction.add_operation(Operation::CreateLink {
    target: PathBuf::from("/home/user/target2"),
    link: PathBuf::from("/home/user/link2"),
});

// Execute atomically - rolls back on any failure
transaction.execute(&fs)?;
```

## Testing Strategy

### Unit Tests
Each module has internal unit tests using `MemoryFileSystem`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::MemoryFileSystem;

    #[test]
    fn test_compute_plan_empty() {
        let desired = LinkMap::new();
        let fs = MemoryFileSystem::new();
        let plan = StateManager::compute_plan(&desired, &fs);
        assert!(plan.is_empty());
    }
}
```

### Integration Tests
Located in `tests/` directory (require running `cargo test`):

- **declarative_tests.rs** - Tests declarative state management
- **atomic_tests.rs** - Tests transaction rollback
- **idempotent_tests.rs** - Tests idempotent operations

**Note:** Integration tests require dependencies that may not be available in all environments.

## Migration Path

### Phase 1: New Architecture (Current)
- New modules alongside old code
- `link_new.rs` coexists with `link.rs`
- Tests written but may not run without dependencies

### Phase 2: Main Refactor (Next)
- Update `main.rs` to use new architecture
- Replace old imperative commands with declarative StateManager
- Keep backward compatibility

### Phase 3: Cleanup
- Remove old `link.rs`
- Rename `link_new.rs` to `link.rs`
- Remove deprecated code

## Benefits of New Architecture

1. **Safety**
   - No panics - all errors handled
   - Path traversal protection
   - Validation before operations

2. **Reliability**
   - Atomic operations - no partial state
   - Rollback on failure
   - Idempotent - safe to retry

3. **Testability**
   - FileSystem abstraction enables testing
   - Unit tests don't touch real filesystem
   - Clear separation of concerns

4. **Maintainability**
   - Clear module structure
   - Each component has single responsibility
   - Easy to extend and modify

5. **User Experience**
   - Declarative - describe what you want
   - Idempotent - safe to run multiple times
   - Atomic - either works completely or not at all

## Future Enhancements

### Logging
```rust
pub struct StateManager {
    logger: Box<dyn Logger>,
}
```

### Parallel Operations
```rust
pub async fn apply_parallel(desired: &LinkMap, fs: Arc<dyn FileSystem>) -> Result<()>
```

### Event System
```rust
pub trait EventHandler {
    fn on_link_created(&self, link: &Link);
    fn on_link_updated(&self, link: &Link);
}
```

### Advanced Validation
```rust
impl Link {
    pub fn validate_with_rules(&self, rules: &[ValidationRule]) -> Result<()>;
}
```

## Comparison: Old vs New

| Aspect | Old | New |
|--------|-----|-----|
| Error Handling | `.unwrap()`, panics | `Result<T, SlmapError>` |
| Operations | Imperative commands | Declarative state |
| Atomicity | No rollback | Full transaction support |
| Idempotence | Not guaranteed | Guaranteed |
| Testing | No abstractions | FileSystem trait |
| Modularity | 2 files | 7 modules |
| Safety | Path traversal possible | Validated |
| Type Safety | `i32` for counts | `usize` for counts |

## Conclusion

The refactored architecture transforms slmap from a simple imperative tool into a robust, production-ready system with:

- **Declarative configuration** - Describe desired state
- **Atomic transactions** - All-or-nothing with rollback
- **Idempotent operations** - Safe to run multiple times
- **Proper error handling** - No panics, rich error context
- **Full testability** - Trait abstractions and unit tests
- **Clear separation of concerns** - Modular design

This provides a solid foundation for future enhancements while maintaining reliability and safety.

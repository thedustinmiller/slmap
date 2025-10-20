# slmap

[![Tests](https://github.com/thedustinmiller/slmap/actions/workflows/test.yml/badge.svg)](https://github.com/thedustinmiller/slmap/actions/workflows/test.yml)
[![CI](https://github.com/thedustinmiller/slmap/actions/workflows/ci.yml/badge.svg)](https://github.com/thedustinmiller/slmap/actions/workflows/ci.yml)

## Declarative Symbolic Link Manager

A robust, declarative utility to manage symbolic links through configuration files. Centralize your dotfiles in a repository and maintain them with atomic, idempotent operations.

### ✨ Key Features

- **Declarative**: Define desired state, let slmap figure out the changes
- **Atomic**: All operations succeed together or fail together (with automatic rollback)
- **Idempotent**: Safe to run multiple times - always converges to desired state
- **Type-safe**: Built with Rust for reliability and performance
- **Tested**: 41 unit tests with comprehensive coverage

## usage  
slmap <command> --map map.toml \
commands are create, update, clean, and status \
create will create all the symlinks if there are no existing conflicting files \
update will update -only- symlinks to point at new target, or create new symlinks if they don't exist \
status will go through the map and check the statuses of symlinks. The possible statuses are:
- Correct
- Incorrect
- NotSymlink
- Missing
- Error \
unexpected statuses will result in no writes, and will print the statuses \
clean deletes all the symlinks in map.toml

Note: the links are relative to where the slmap command is run. Paths are interpreted with shell variables and ~

## Architecture

See [ARCHITECTURE.md](ARCHITECTURE.md) for comprehensive architecture documentation.

The refactored design implements three core guarantees:
1. **Declarative** - Configuration describes desired state
2. **Atomic** - Transactions with automatic rollback on failure
3. **Idempotent** - Running multiple times produces same result

## Testing

Run tests with:
```bash
cargo test
```

See [TEST_COVERAGE.md](TEST_COVERAGE.md) for detailed test documentation.

**Test Coverage**: 41 unit tests across all modules
- FileSystem: 9 tests
- Transactions: 14 tests
- State Manager: 15 tests
- Links: 3 tests

## Documentation

- [ARCHITECTURE.md](ARCHITECTURE.md) - Comprehensive architecture guide
- [REFACTOR.md](REFACTOR.md) - Refactor summary and comparison
- [TEST_COVERAGE.md](TEST_COVERAGE.md) - Test coverage report

file format:  
```toml
[filename]  
target = 'path/to/file'  
link_name = 'path/to/file'

[directory]
target = 'config/dir'
link_name = 'test/dir'
directory = true

[zshrc]  
target = 'config/zshrc'  
link_name = 'test/.zshrc'  
  
[vimrc]  
target = 'config/vimrc'  
link_name = 'test/.vimrc'   

[motd]
target = 'config/motd'
link_name = '/etc/motd'
```

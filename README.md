# slmap
## symbolic link map

Simple utility to manage a list of symlinks, helping you centralize your config files in a repository and symlink to them. `slmap` operates on a "per-link" success or failure basis, aiming for non-destructive actions by default.

## Usage
`slmap <command> --map map.toml`

Paths in the map file are interpreted with shell variables (e.g., `$HOME`) and `~` for the home directory. Link operations are relative to where the `slmap` command is run, unless absolute paths are specified in the `map.toml`.

### Commands

**`create`**
- Attempts to create each symlink defined in your `map.toml`.
- If a `link_name` path does not exist, the symlink will be created.
- If `link_name` already exists as a symlink and points to the correct `target`, the operation is considered successful for that link.
- **Non-destructive**: `create` will **not** overwrite or modify existing files, directories, or symlinks that point to an incorrect target at the `link_name` path. Instead, it will report an error for each such conflicting link and proceed with other links.

**`update`**
- This command acts as an "ensure all links are present and correct" operation.
- It iterates through the map and attempts to establish each link:
    - If a link is missing, it will be created.
    - If a link exists and is correct, it's confirmed (no change).
    - If `link_name` is occupied by a regular file, a directory, or an incorrect symlink, an error will be reported for that specific link, and it will be skipped. Other links will still be processed.

**`clean`**
- Attempts to remove each symlink defined in your `map.toml`.
- **Only removes symlinks**: If a `link_name` path points to an actual symlink, it will be deleted.
- If `link_name` path points to a regular file or a directory, `clean` will not remove it and will report an error for that entry.

**`status`**
- Checks and reports the current state of each link defined in `map.toml`. The possible statuses are:
    - `Correct`: The symlink exists at `link_name` and points to the specified `target`.
    - `Missing`: The symlink does not exist at `link_name`.
    - `Incorrect`: A symlink exists at `link_name`, but it points to a different target than specified.
    - `Conflict`: The path at `link_name` exists but is a regular file or a directory, not a symlink.

### File Format Example
```toml
[filename]
target = 'path/to/target_file' # Can be relative to slmap execution or absolute
link_name = 'path/to/link_location' # Can be relative or absolute

[directory_example]
target = 'config/my_app_config_dir' # Target is a directory
link_name = 'test/my_app_config_dir_link'
# 'directory = true' is no longer used in the Link struct itself for link creation logic,
# but can be kept for user clarity if desired. The tool infers target type.

[zshrc_home]  
target = 'config/zshrc'  # Assumes 'config/zshrc' is in the same dir as slmap or an absolute path
link_name = '~/.zshrc' # Creates a symlink in the home directory
  
[vimrc_example]  
target = 'config/vimrc'   
link_name = 'test/.vimrc' # Creates a symlink named .vimrc in the 'test' subdirectory

[motd_system]
target = 'config/motd_content' # Your custom motd file
link_name = '/etc/motd' # System file, requires appropriate permissions
# 'root = true' is no longer used in the Link struct. Permissions are handled by the OS.
```

## Future Goals (Archive - some might be implicitly addressed by new design)
- Handling permissions (now mostly an OS-level concern for the user running `slmap`)
- More nuanced error reporting (current model reports per-link errors)
- Checking in a new file to the map (this is a workflow outside `slmap`'s direct operation)
  - Swap out the file into the repository and update the map manually.

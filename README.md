# slmap
## symbolic link map

Simple utility to manage a list of symlinks so you can centralize your config files in a repo and symlink to them.

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

Note: the links are relative to where the slmap command is run. Paths are interpretted with shell variables and ~

Future goals:
- actually handling permissions
- error handling nicely vs just panics
- checking in a new file to the map
  - swap out the txt into the repository and update the map
- 

file format:  
```toml
[filename]  
target = 'path/to/file'  
link_name = 'path/to/file'  
root = true/false

[zshrc]  
target = 'config/zshrc'  
link_name = 'test/.zshrc'  
  
[vimrc]  
target = 'config/vimrc'  
link_name = 'test/.vimrc'  
  
[folder]  
target = 'config/folder'  
link_name = 'test/folder'

[motd]
target = 'config/motd'
link_name = '/etc/motd'
root = true
```

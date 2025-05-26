use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    os::unix,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl Link {
    pub fn resolved_link_name(&self) -> Result<PathBuf, String> {
        match resolve_path(self.link_name.clone()) {
            Ok(p) => Ok(p),
            Err(e) => Err(e),
        }
    }

    pub fn resolved_target(&self) -> Result<PathBuf, String> {
        match resolve_path(self.target.clone()) {
            Ok(p) => Ok(p),
            Err(e) => Err(e),
        }
    }

    pub fn create_link(&self) -> Result<(), String> {
        let target = self.resolved_target()?;
        let link_name = self.resolved_link_name()?;

        if !link_name.exists() {
            // Create parent directories if they don't exist
            if let Some(parent) = link_name.parent() {
                if !parent.exists() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create parent directories: {}", e))?;
                }
            }
            // Create the symlink
            unix::fs::symlink(&target, &link_name)
                .map_err(|e| format!("Failed to create symlink: {}", e))?;
            Ok(())
        } else {
            // Handle existing link_name
            if link_name.is_symlink() {
                match fs::read_link(&link_name) {
                    Ok(existing_target) => {
                        if existing_target == target {
                            // Symlink already exists and points to the correct target
                            Ok(())
                        } else {
                            Err(format!(
                                "Link '{}' already exists but points to '{}' instead of '{}'",
                                link_name.display(),
                                existing_target.display(),
                                target.display()
                            ))
                        }
                    }
                    Err(e) => Err(format!("Failed to read existing symlink: {}", e)),
                }
            } else if link_name.is_dir() {
                Err(format!(
                    "Cannot create link: '{}' already exists and is a directory.",
                    link_name.display()
                ))
            } else if link_name.is_file() {
                Err(format!(
                    "Cannot create link: '{}' already exists and is a file.",
                    link_name.display()
                ))
            } else {
                Err(format!(
                    "Cannot create link: '{}' already exists and is not a symlink, directory, or file.",
                    link_name.display()
                ))
            }
        }
    }

    pub fn delete_link(&self) -> Result<(), String> {
        let link_name = self.resolved_link_name().unwrap();
        if link_name.is_symlink() {
            match fs::remove_file(link_name.as_path()) {
                Ok(_) => Ok(()),
                Err(e) => Err(e.to_string()),
            }
        } else {
            Err("Not a symlink".to_string())
        }
    }
}

fn resolve_path(path: String) -> Result<PathBuf, String> {
    match shellexpand::full(&path) {
        Ok(p) => Ok(PathBuf::from(p.into_owned())),
        Err(e) => Err(e.var_name),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix;
    use std::path::PathBuf;

    const TEST_BASE_DIR: &str = "test_assets_create_link";

    // Helper function to create a dummy file
    fn create_dummy_file(path: &PathBuf) {
        fs::File::create(path)
            .unwrap_or_else(|e| panic!("Failed to create dummy file at {:?}: {}", path, e));
    }

    // Helper function to create a dummy directory
    fn create_dummy_dir(path: &PathBuf) {
        fs::create_dir_all(path)
            .unwrap_or_else(|e| panic!("Failed to create dummy dir at {:?}: {}", path, e));
    }

    // Setup function to create and clean the base test directory for each test
    fn setup_test_dir(test_name: &str) -> PathBuf {
        let test_dir = PathBuf::from(TEST_BASE_DIR).join(test_name);
        if test_dir.exists() {
            fs::remove_dir_all(&test_dir)
                .unwrap_or_else(|e| panic!("Failed to clean up test dir {:?}: {}", test_dir, e));
        }
        fs::create_dir_all(&test_dir)
            .unwrap_or_else(|e| panic!("Failed to create test dir {:?}: {}", test_dir, e));
        test_dir
    }

    // Cleanup function to remove the base test directory
    fn cleanup_test_base_dir() {
        let base_dir = PathBuf::from(TEST_BASE_DIR);
        if base_dir.exists() {
            fs::remove_dir_all(&base_dir).expect("Failed to remove test_assets_create_link directory");
        }
    }

    #[test]
    fn test_create_link_target_does_not_exist() {
        let test_dir = setup_test_dir("target_does_not_exist");
        let target_path = test_dir.join("non_existent_target.txt");
        let link_path = test_dir.join("link_to_non_existent");

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_ok(), "create_link should succeed even if target doesn't exist. Result: {:?}", result);
        assert!(link_path.exists(), "Link path should exist.");
        assert!(fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Link path should be a symlink.");
        assert_eq!(fs::read_link(&link_path).unwrap(), target_path, "Symlink should point to the specified target path.");

        // Cleanup (test_dir is removed by cleanup_test_base_dir if tests are run sequentially,
        // but explicit removal is safer for isolated test runs)
        fs::remove_dir_all(test_dir).unwrap();
        // Call cleanup_test_base_dir() at the end of all tests or use a test runner feature if available
    }

    #[test]
    fn test_create_link_link_name_does_not_exist() {
        let test_dir = setup_test_dir("link_name_does_not_exist");
        let target_path = test_dir.join("target.txt");
        create_dummy_file(&target_path);
        let link_path = test_dir.join("link_to_target");

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_ok(), "create_link failed: {:?}", result.err());
        assert!(link_path.exists(), "Link path should exist.");
        assert!(fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Link path should be a symlink.");
        assert_eq!(fs::read_link(&link_path).unwrap(), target_path, "Symlink does not point to the correct target.");

        fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_create_link_link_name_is_correct_symlink() {
        let test_dir = setup_test_dir("link_name_is_correct_symlink");
        let target_path = test_dir.join("target.txt");
        create_dummy_file(&target_path);
        let link_path = test_dir.join("link_to_target");
        unix::fs::symlink(&target_path, &link_path).unwrap();

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_ok(), "create_link failed for an already correct symlink: {:?}", result.err());
        assert!(link_path.exists(), "Link path should still exist.");
        assert!(fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Link path should still be a symlink.");
        assert_eq!(fs::read_link(&link_path).unwrap(), target_path, "Symlink does not point to the correct target.");

        fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_create_link_link_name_is_file() {
        let test_dir = setup_test_dir("link_name_is_file");
        let target_path = test_dir.join("target.txt");
        create_dummy_file(&target_path);
        let link_path = test_dir.join("link_location_is_file");
        create_dummy_file(&link_path); // Create a regular file at link_path

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_err(), "create_link should have returned an error.");
        assert!(link_path.exists(), "File at link_path should still exist.");
        assert!(fs::metadata(&link_path).unwrap().is_file(), "Path should still be a regular file.");
        assert!(!fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Path should not be a symlink.");

        fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_create_link_link_name_is_dir() {
        let test_dir = setup_test_dir("link_name_is_dir");
        let target_path = test_dir.join("target.txt"); // Target can be a file for this test
        create_dummy_file(&target_path);
        let link_path_as_dir = test_dir.join("link_location_is_dir");
        create_dummy_dir(&link_path_as_dir); // Create a directory at link_path

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path_as_dir.to_str().unwrap().to_string(),
            directory: false, // Field 'directory' in Link refers to target type, not link_name
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_err(), "create_link should have returned an error when link_name is a directory.");
        assert!(link_path_as_dir.exists(), "Directory at link_path_as_dir should still exist.");
        assert!(fs::metadata(&link_path_as_dir).unwrap().is_dir(), "Path should still be a directory.");

        fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_create_link_link_name_is_incorrect_symlink() {
        let test_dir = setup_test_dir("link_name_is_incorrect_symlink");
        let target_actual_path = test_dir.join("actual_target.txt");
        create_dummy_file(&target_actual_path);
        let target_intended_path = test_dir.join("intended_target.txt");
        create_dummy_file(&target_intended_path);
        let link_path = test_dir.join("link_to_actual");
        unix::fs::symlink(&target_actual_path, &link_path).unwrap();

        let link_obj = Link {
            target: target_intended_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_err(), "create_link should have returned an error for an incorrect symlink.");
        assert!(link_path.exists(), "Link path should still exist.");
        assert!(fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Link path should still be a symlink.");
        assert_eq!(fs::read_link(&link_path).unwrap(), target_actual_path, "Symlink should still point to the actual (original) target.");

        fs::remove_dir_all(test_dir).unwrap();
    }

    #[test]
    fn test_create_link_parent_dir_does_not_exist() {
        let test_dir_base = setup_test_dir("parent_dir_does_not_exist_base");
        let target_path = test_dir_base.join("target.txt");
        create_dummy_file(&target_path);

        // Define a link_name within a subdirectory that doesn't exist yet
        let parent_dir = test_dir_base.join("non_existent_parent");
        let link_path = parent_dir.join("my_link");

        let link_obj = Link {
            target: target_path.to_str().unwrap().to_string(),
            link_name: link_path.to_str().unwrap().to_string(),
            directory: false,
            root: false,
        };

        let result = link_obj.create_link();
        assert!(result.is_ok(), "create_link failed when parent directory does not exist: {:?}", result.err());
        assert!(parent_dir.exists(), "Parent directory should have been created.");
        assert!(parent_dir.is_dir(), "Parent path should be a directory.");
        assert!(link_path.exists(), "Link path should exist.");
        assert!(fs::symlink_metadata(&link_path).unwrap().file_type().is_symlink(), "Link path should be a symlink.");
        assert_eq!(fs::read_link(&link_path).unwrap(), target_path, "Symlink does not point to the correct target.");

        fs::remove_dir_all(test_dir_base).unwrap();
    }
    
    // It's good practice to have a way to clean up the top-level test directory
    // if many tests are run. This could be part of a test suite teardown.
    // For now, individual test cleanups are done. If running all tests,
    // this ensures the TEST_BASE_DIR is removed once at the end.
    // This is tricky to do with `#[test]` functions alone without a custom test harness.
    // We can call it at the end of the "suite" if we run tests serially or manually.
    // For robustness, each test cleans its own specific sub-directory.
    // A final cleanup function can be registered with `std::rt::at_exit` if needed for standalone runs,
    // but that's often overkill for library tests.
    // The current setup_test_dir cleans up previous runs of the same test, which is good.
}

use assert_cmd::prelude::*; // Add methods on commands
use predicates::prelude::*; // Used for writing assertions
use std::process::Command; // Run programs
use std::fs;
use std::path::{Path, PathBuf};
use std::io::Write; // To write to map_file
use tempfile::tempdir; // For temporary directories per test

// Helper to create a map.toml file
fn create_map_file(dir: &Path, content: &str) -> PathBuf {
    let map_path = dir.join("map.toml");
    let mut file = fs::File::create(&map_path).expect("Failed to create map.toml");
    writeln!(file, "{}", content).expect("Failed to write to map.toml");
    map_path
}

// Helper to create a dummy file or directory
// Ensures parent directories are created for files as well.
fn create_fs_entry(path: &Path, is_dir: bool) {
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).expect(&format!("Failed to create parent dir for {:?}", path));
        }
    }
    if is_dir {
        if !path.exists() { // Check before creating to avoid error if parent creation also created this
            fs::create_dir_all(path).expect(&format!("Failed to create dir at {:?}", path));
        }
    } else {
        fs::File::create(path).expect(&format!("Failed to create file at {:?}", path));
    }
}

#[test]
fn test_create_command_simple() { // This is the example, will adapt for required tests
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link1]
target = "target_file.txt"
link_name = "link_to_file"
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Create the target file relative to base_path
    create_fs_entry(&base_path.join("target_file.txt"), false);

    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path) // Run slmap from the temporary directory
        .arg("create")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap()) // Use relative path for map file
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully created link 'link1'"));

    let link_path = base_path.join("link_to_file");
    assert!(link_path.exists(), "Link path should exist");
    assert!(fs::symlink_metadata(&link_path).expect("Failed to get symlink metadata").file_type().is_symlink(), "Path should be a symlink");
    // Ensure the symlink target is also relative to base_path if appropriate, or absolute.
    // For targets defined relatively in map.toml, read_link will return that relative path.
    assert_eq!(fs::read_link(&link_path).unwrap(), PathBuf::from("target_file.txt"));
}

// Test case a: test_create_command_non_destructive
#[test]
fn test_create_command_non_destructive() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link_A]
target = "target_A.txt"
link_name = "link_A_name"

[link_B]
target = "target_B.txt" # Target for link_B, existence doesn't matter for this test focus
link_name = "link_B_name_conflict" 
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Setup for link_A
    create_fs_entry(&base_path.join("target_A.txt"), false);

    // Setup for link_B (conflict: create a regular file where symlink should be)
    create_fs_entry(&base_path.join("link_B_name_conflict"), false);
    let pre_conflict_metadata = fs::metadata(base_path.join("link_B_name_conflict")).unwrap();


    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path)
        .arg("create")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap())
        .assert()
        // The command overall might be considered successful by assert_cmd if exit code is 0,
        // even if some links fail. The output check is more critical.
        .success() // Assuming partial success is still exit code 0
        .stdout(predicate::str::contains("Successfully created link 'link_A'"))
        .stdout(predicate::str::contains("Failed to create link 'link_B'")
        .and(predicate::str::contains("already exists and is a file")));


    // Assertions for link_A
    let link_a_path = base_path.join("link_A_name");
    assert!(link_a_path.exists(), "link_A should exist");
    assert!(fs::symlink_metadata(&link_a_path).unwrap().file_type().is_symlink(), "link_A should be a symlink");
    assert_eq!(fs::read_link(&link_a_path).unwrap(), PathBuf::from("target_A.txt"));

    // Assertions for link_B (should be untouched)
    let link_b_path = base_path.join("link_B_name_conflict");
    assert!(link_b_path.exists(), "Conflicting file for link_B should still exist");
    assert!(fs::metadata(&link_b_path).unwrap().is_file(), "Conflicting path for link_B should still be a file.");
    let post_conflict_metadata = fs::metadata(link_b_path).unwrap();
    assert_eq!(pre_conflict_metadata.len(), post_conflict_metadata.len(), "File content/metadata should not have changed");
    assert_eq!(pre_conflict_metadata.modified().unwrap(), post_conflict_metadata.modified().unwrap(), "File modification time should not have changed");
}

// Test case b: test_create_command_already_exists_correctly
#[test]
fn test_create_command_already_exists_correctly() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link_C]
target = "target_C.txt"
link_name = "link_C_name"
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Setup for link_C
    let target_c_path_abs = base_path.join("target_C.txt");
    create_fs_entry(&target_c_path_abs, false);

    let link_c_path_abs = base_path.join("link_C_name");
    // Manually create the correct symlink
    std::os::unix::fs::symlink("target_C.txt", &link_c_path_abs).expect("Failed to create symlink for test setup");

    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path)
        .arg("create")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap())
        .assert()
        .success()
        // The message "Link 'link_C' is up to date or successfully created." comes from the update function
        // The create function in link.rs returns Ok(()) if link is correct.
        // main.rs create function prints "Successfully created link 'link_C'" for Ok result.
        .stdout(predicate::str::contains("Successfully created link 'link_C'"));


    assert!(link_c_path_abs.exists(), "link_C should still exist");
    assert!(fs::symlink_metadata(&link_c_path_abs).unwrap().file_type().is_symlink(), "link_C should still be a symlink");
    assert_eq!(fs::read_link(&link_c_path_abs).unwrap(), PathBuf::from("target_C.txt"));
}

// Test case c: test_clean_command
#[test]
fn test_clean_command() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link_D]
target = "target_D.txt"
link_name = "link_D_name"

[link_E]
target = "target_E.txt"
link_name = "link_E_name"
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Setup for link_D
    create_fs_entry(&base_path.join("target_D.txt"), false);
    std::os::unix::fs::symlink("target_D.txt", base_path.join("link_D_name")).unwrap();

    // Setup for link_E
    create_fs_entry(&base_path.join("target_E.txt"), false);
    std::os::unix::fs::symlink("target_E.txt", base_path.join("link_E_name")).unwrap();

    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path)
        .arg("clean")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("Successfully deleted link 'link_D'"))
        .stdout(predicate::str::contains("Successfully deleted link 'link_E'"));

    assert!(!base_path.join("link_D_name").exists(), "link_D_name should be removed");
    assert!(!fs::symlink_metadata(&base_path.join("link_D_name")).is_ok(), "link_D_name should be removed, symlink_metadata check should fail");
    assert!(!base_path.join("link_E_name").exists(), "link_E_name should be removed");
    assert!(!fs::symlink_metadata(&base_path.join("link_E_name")).is_ok(), "link_E_name should be removed, symlink_metadata check should fail");
}

// Test case d: test_clean_command_non_symlink
#[test]
fn test_clean_command_non_symlink() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link_F]
target = "target_F.txt" # Target existence doesn't matter here
link_name = "link_F_name_is_file"
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Setup for link_F (create a regular file instead of a symlink)
    let file_path = base_path.join("link_F_name_is_file");
    create_fs_entry(&file_path, false);
    let file_content_before = "Hello I am a file";
    fs::write(&file_path, file_content_before).unwrap();


    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path)
        .arg("clean")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap())
        .assert()
        .success() // Clean command in main.rs prints error but doesn't exit with error code
        .stderr(predicate::str::contains("Failed to delete link 'link_F': Not a symlink"));

    assert!(file_path.exists(), "File link_F_name_is_file should still exist");
    assert!(fs::metadata(&file_path).unwrap().is_file(), "link_F_name_is_file should still be a file");
    assert_eq!(fs::read_to_string(&file_path).unwrap(), file_content_before, "File content should not have changed.");
}

// Test case e: test_status_command
#[test]
fn test_status_command() {
    let temp_dir = tempdir().unwrap();
    let base_path = temp_dir.path();

    let map_content = r#"
[link_correct]
target = "target_correct.txt"
link_name = "link_correct_name"

[link_missing_link]
target = "target_for_missing_link.txt"
link_name = "link_missing_link_name"

[link_missing_target] # This is a broken link
target = "target_missing.txt" 
link_name = "link_missing_target_name"

[link_incorrect_symlink]
target = "target_intended_for_incorrect.txt"
link_name = "link_incorrect_symlink_name"

[link_is_file]
target = "target_for_file_conflict.txt"
link_name = "link_is_file_name"
"#;
    let map_file_path = create_map_file(base_path, map_content);

    // Setup:
    // 1. link_correct
    create_fs_entry(&base_path.join("target_correct.txt"), false);
    std::os::unix::fs::symlink("target_correct.txt", base_path.join("link_correct_name")).unwrap();

    // 2. link_missing_link
    create_fs_entry(&base_path.join("target_for_missing_link.txt"), false);
    // ... link_missing_link_name is not created

    // 3. link_missing_target
    // ... target_missing.txt is not created
    std::os::unix::fs::symlink("target_missing.txt", base_path.join("link_missing_target_name")).unwrap();

    // 4. link_incorrect_symlink
    create_fs_entry(&base_path.join("target_intended_for_incorrect.txt"), false);
    create_fs_entry(&base_path.join("actual_different_target.txt"), false);
    std::os::unix::fs::symlink("actual_different_target.txt", base_path.join("link_incorrect_symlink_name")).unwrap();
    
    // 5. link_is_file
    create_fs_entry(&base_path.join("target_for_file_conflict.txt"), false); // Target for completeness
    create_fs_entry(&base_path.join("link_is_file_name"), false); // Actual file where link should be

    Command::cargo_bin("slmap").unwrap()
        .current_dir(base_path)
        .arg("status")
        .arg("--map")
        .arg(map_file_path.file_name().unwrap().to_str().unwrap())
        .assert()
        .success()
        .stdout(predicate::str::contains("link_correct").and(predicate::str::contains("Correct")))
        .stdout(predicate::str::contains("link_missing_link").and(predicate::str::contains("Missing")))
        .stdout(predicate::str::contains("link_missing_target").and(predicate::str::contains("Incorrect").or(predicate::str::contains("Error")))) // Broken links are 'Incorrect' because read_link target != resolved target path. If target_missing.txt doesn't exist, it's still an existing symlink.
        .stdout(predicate::str::contains("link_incorrect_symlink").and(predicate::str::contains("Incorrect").and(predicate::str::contains("actual_different_target.txt"))))
        .stdout(predicate::str::contains("link_is_file").and(predicate::str::contains("Conflict").and(predicate::str::contains("Exists as a file"))));

}

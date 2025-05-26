mod link;

use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    // os::unix, // Not strictly needed in main.rs after refactor
    path::{Path, PathBuf},
};

use clap::{self, Arg, ArgAction, Command};
use colored::*;
use link::Link; // Removed LinkStatus
use serde::{Deserialize, Serialize}; // Serialize not strictly needed in main.rs

// Statuses struct and statuses function removed

fn read_map(map_file: &mut File) -> HashMap<String, Link> {
    let mut file_string = String::new();
    map_file
        .read_to_string(&mut file_string)
        .expect("read fail");

    let map: HashMap<String, Link> = toml::from_str(&file_string).unwrap();

    map
}

fn create(map: &HashMap<String, Link>) {
    for (name, link) in map {
        println!("Attempting to create link '{}': {} -> {}", name, link.link_name, link.target);
        match link.create_link() {
            Ok(()) => println!("Successfully created link '{}'", name),
            Err(e) => eprintln!("Failed to create link '{}': {}", name, e),
        }
    }
}

fn clean(map: &HashMap<String, Link>) {
    for (name, link) in map {
        println!("Attempting to delete link '{}': {}", name, link.link_name);
        match link.delete_link() {
            Ok(()) => println!("Successfully deleted link '{}'", name),
            Err(e) => eprintln!("Failed to delete link '{}': {}", name, e),
        }
    }
}

fn update(map: &HashMap<String, Link>) {
    println!("Updating links...");
    for (name, link) in map {
        println!("Attempting to create/update link '{}': {} -> {}", name, link.link_name, link.target);
        match link.create_link() {
            Ok(()) => println!("Link '{}' is up to date or successfully created.", name),
            Err(e) => eprintln!("Failed to create/update link '{}': {}", name, e),
        }
    }
}

// Old dry-run and print_statuses functions removed.

fn clean_dry_run(map: &HashMap<String, Link>) {
    println!("Dry run: Cleaning links...");
    for (name, link) in map {
        println!("Would attempt to delete link '{}': {}", name, link.link_name);
    }
}

fn status(map: &HashMap<String, Link>) {
    println!("Checking link statuses...");
    for (name, link_def) in map {
        // It's better to resolve paths once and handle potential errors from resolution.
        let link_path_res = link_def.resolved_link_name();
        let target_path_res = link_def.resolved_target();

        match (link_path_res, target_path_res) {
            (Ok(link_path), Ok(target_path)) => {
                print!("Link '{}' ({} -> {}): ", name, link_def.link_name, link_def.target);
                if !link_path.exists() {
                    // Check if the target exists before declaring missing,
                    // as a missing target for a non-existent link is just "Missing".
                    // If target itself is missing, `create_link` would also fail if it tried to link it.
                    println!("{}", "Missing".yellow());
                } else if link_path.is_symlink() {
                    match fs::read_link(&link_path) {
                        Ok(actual_target) => {
                            if actual_target == target_path {
                                println!("{}", "Correct".green());
                            } else {
                                println!("{} (points to {})", "Incorrect".red(), actual_target.display());
                            }
                        }
                        Err(e) => println!("{} (Error reading link: {})", "Error".red(), e),
                    }
                } else if link_path.is_file() {
                    println!("{} (Exists as a file)", "Conflict".red());
                } else if link_path.is_dir() {
                    println!("{} (Exists as a directory)", "Conflict".red());
                } else {
                    println!("{} (Exists as something else)", "Conflict".red());
                }
            }
            (Err(e), _) => eprintln!("Error resolving link name for '{}': {}", name, e),
            (_, Err(e)) => eprintln!("Error resolving target path for '{}': {}", name, e),
        }
    }
}

fn main() {
    let matches = Command::new("slmap")
        .about("symlink manager")
        .version("0.2.0")
        .arg_required_else_help(true)
        .author("Dustin Miller")
        .arg(
            Arg::new("command")
                .help("which command to run")
                .value_parser(["create", "update", "status", "clean"]),
        )
        .arg(
            Arg::new("map_file")
                .short('m')
                .long("map")
                .help("Map file location")
                .default_value("map.toml"),
        )
        .arg(
            Arg::new("dry-run")
                .short('d')
                .long("dry-run")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let command = matches.get_one::<String>("command").unwrap();
    let dry_run = matches.get_flag("dry-run");
    let map_file_string = matches.get_one::<String>("map_file").unwrap();

    let mut map_file = OpenOptions::new()
        .read(true)
        .open(map_file_string)
        .expect("Unable to open map file");

    let map = read_map(&mut map_file);

    match command.as_str() {
        "create" => {
            // Dry run for create is removed as per plan.
            // The new create function is inherently non-destructive and reports actions.
            create(&map);
        }
        "update" => {
            // Dry run for update is removed as per plan.
            // The new update function is inherently non-destructive and reports actions.
            update(&map);
        }
        "status" => {
            status(&map); // Call the new status function
        }
        "clean" => {
            if dry_run {
                clean_dry_run(&map); // Keep dry-run for clean
            } else {
                clean(&map);
            }
        }
        _ => {
            // This case should ideally be handled by clap's value_parser
            // but as a fallback:
            eprintln!("Invalid command: {}", command);
            // panic!("Invalid command"); // Or exit more gracefully
        }
    }
}

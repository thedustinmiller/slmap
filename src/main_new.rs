// Example of refactored main.rs using the new architecture
// This demonstrates how to use the declarative, atomic, and idempotent design
//
// To use this: rename main.rs to main_old.rs and this file to main.rs

use clap::{Arg, ArgAction, Command};
use colored::*;
use slmap::{
    error::Result, Change, ExecutionPlan, Link, LinkMap, LinkStatus, RealFileSystem,
    StateManager,
};
use std::fs::File;
use std::io::Read;
use std::process;

fn main() {
    let matches = Command::new("slmap")
        .about("Declarative symlink manager")
        .version(env!("CARGO_PKG_VERSION"))  // Auto-synced with Cargo.toml
        .arg_required_else_help(true)
        .author("Dustin Miller")
        .arg(
            Arg::new("command")
                .help("Command to run: apply, status, validate")
                .value_parser(["apply", "status", "validate"]),
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
                .action(ArgAction::SetTrue)
                .help("Show what would be done without making changes"),
        )
        .get_matches();

    let command = matches.get_one::<String>("command").unwrap();
    let dry_run = matches.get_flag("dry-run");
    let map_file_path = matches.get_one::<String>("map_file").unwrap();

    // Load configuration
    let desired = match load_config(map_file_path) {
        Ok(config) => config,
        Err(e) => {
            eprintln!("{} {}", "Error:".red().bold(), e);
            process::exit(1);
        }
    };

    let fs = RealFileSystem;

    // Execute command
    let result = match command.as_str() {
        "apply" => cmd_apply(&desired, &fs, dry_run),
        "status" => cmd_status(&desired, &fs),
        "validate" => cmd_validate(&desired, &fs),
        _ => unreachable!(),
    };

    if let Err(e) = result {
        eprintln!("{} {}", "Error:".red().bold(), e);
        process::exit(1);
    }
}

/// Load and parse TOML configuration
fn load_config(path: &str) -> Result<LinkMap> {
    let mut file = File::open(path)
        .map_err(|e| slmap::error::SlmapError::Io(e))?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .map_err(|e| slmap::error::SlmapError::Io(e))?;

    let map: LinkMap = toml::from_str(&contents)
        .map_err(|e| slmap::error::SlmapError::TomlParse(e))?;

    Ok(map)
}

/// Apply command - declaratively apply desired state
fn cmd_apply(
    desired: &LinkMap,
    fs: &RealFileSystem,
    dry_run: bool,
) -> Result<()> {
    // Compute what changes are needed
    let plan = StateManager::compute_plan(desired, fs);

    if plan.is_empty() {
        println!("{}", "✓ System is already in sync - no changes needed".green());
        return Ok(());
    }

    // Show plan
    print_plan(&plan);

    if dry_run {
        println!("\n{}", "Dry run - no changes made".yellow());
        return Ok(());
    }

    // Apply changes atomically
    println!("\n{}", "Applying changes...".bold());

    match StateManager::apply(desired, fs) {
        Ok(_) => {
            println!("{}", "✓ Successfully applied all changes".green().bold());

            // Verify state is synced
            if StateManager::is_synced(desired, fs) {
                println!("{}", "✓ System is now in sync".green());
            }

            Ok(())
        }
        Err(e) => {
            eprintln!(
                "{}",
                "✗ Failed to apply changes - all changes rolled back".red().bold()
            );
            Err(e)
        }
    }
}

/// Status command - show current state vs desired state
fn cmd_status(desired: &LinkMap, fs: &RealFileSystem) -> Result<()> {
    println!("{}", "Checking link status...\n".bold());

    let plan = StateManager::compute_plan(desired, fs);

    if StateManager::is_synced(desired, fs) {
        println!("{}", "✓ All links are correct".green().bold());
        print_summary(&plan);
        return Ok(());
    }

    println!("{}", "Links need attention:".yellow().bold());
    print_plan(&plan);
    print_summary(&plan);

    Ok(())
}

/// Validate command - check configuration without touching filesystem
fn cmd_validate(desired: &LinkMap, fs: &RealFileSystem) -> Result<()> {
    println!("{}", "Validating configuration...\n".bold());

    let errors = StateManager::validate(desired, fs)?;

    if errors.is_empty() {
        println!("{}", "✓ Configuration is valid".green().bold());
        println!("  {} link(s) defined", desired.len());
        Ok(())
    } else {
        println!("{}", "✗ Configuration has errors:".red().bold());
        for error in errors {
            println!("  • {}", error);
        }
        process::exit(1);
    }
}

/// Print execution plan with colors
fn print_plan(plan: &ExecutionPlan) {
    for change in plan.changes() {
        match change {
            Change::Create { name, link } => {
                println!(
                    "  {} {} -> {}",
                    "+".green().bold(),
                    name.cyan(),
                    format!("{} -> {}", link.link_name, link.target).dimmed()
                );
            }
            Change::Update { name, link } => {
                println!(
                    "  {} {} -> {}",
                    "~".yellow().bold(),
                    name.cyan(),
                    format!("{} -> {}", link.link_name, link.target).dimmed()
                );
            }
            Change::Remove { name, link } => {
                println!(
                    "  {} {} -> {}",
                    "-".red().bold(),
                    name.cyan(),
                    format!("{}", link.link_name).dimmed()
                );
            }
            Change::NoChange { name, .. } => {
                println!("  {} {}", "✓".green(), name.cyan());
            }
        }
    }
}

/// Print summary statistics
fn print_summary(plan: &ExecutionPlan) {
    let (creates, updates, removes, no_changes) = plan.summary();

    println!();
    println!("Summary:");
    if creates > 0 {
        println!("  {} to create", creates.to_string().green());
    }
    if updates > 0 {
        println!("  {} to update", updates.to_string().yellow());
    }
    if removes > 0 {
        println!("  {} to remove", removes.to_string().red());
    }
    if no_changes > 0 {
        println!("  {} correct", no_changes.to_string().green());
    }
}

// Example usage in comments:
//
// // Apply configuration (idempotent)
// $ slmap apply -m map.toml
//   + vimrc -> /home/user/.vimrc -> config/vimrc
//   + zshrc -> /home/user/.zshrc -> config/zshrc
//   ✓ tmux -> /home/user/.tmux.conf
//
// Summary:
//   2 to create
//   1 correct
//
// Applying changes...
// ✓ Successfully applied all changes
// ✓ System is now in sync
//
// // Running again does nothing (idempotent)
// $ slmap apply -m map.toml
// ✓ System is already in sync - no changes needed
//
// // Check status without changes
// $ slmap status -m map.toml
// ✓ All links are correct
//
// Summary:
//   3 correct
//
// // Dry run to preview changes
// $ slmap apply -m map.toml --dry-run
//   + nvim -> /home/user/.config/nvim -> config/nvim
//
// Summary:
//   1 to create
//
// Dry run - no changes made

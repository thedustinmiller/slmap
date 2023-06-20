#![allow(unused_variables, unused_imports)]

mod link;

use std::{
	collections::HashMap,
	fs::{self, File, OpenOptions},
	io::{Read, Write},
	os::unix,
	path::{Path, PathBuf},
};

use clap::{self, Arg, Command};
use colored::*;
use link::{Link, LinkStatus};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
struct Statuses<'a> {
	link_tuples: Vec<(&'a String, &'a Link, LinkStatus)>,
	missing: i32,
	not_symlink: i32,
	correct: i32,
	incorrect: i32,
	error: i32,
}

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
		println!("{} -> {}", link.from, link.to);
		link.create_link();
	}
}

fn update(map: &HashMap<String, Link>, lock_map: &HashMap<String, Link>) {}

fn statuses(map: &HashMap<String, Link>) -> Statuses {
	let mut statuses: Vec<(&String, &Link, LinkStatus)> = Vec::new();
	let mut missing = 0;
	let mut not_symlink = 0;
	let mut correct = 0;
	let mut incorrect = 0;
	let mut error = 0;

	for (name, link) in map {
		match link.check_link() {
			LinkStatus::Missing => {
				statuses.push((name, link, LinkStatus::Missing));
				missing += 1;
			}
			LinkStatus::NotSymlink => {
				statuses.push((name, link, LinkStatus::NotSymlink));
				not_symlink += 1;
			}
			LinkStatus::Correct => {
				statuses.push((name, link, LinkStatus::Correct));
				correct += 1;
			}
			LinkStatus::Incorrect(actual_target) => {
				statuses.push((name, link, LinkStatus::Incorrect(actual_target)));
				incorrect += 1;
			}
			LinkStatus::Error(e) => {
				statuses.push((name, link, LinkStatus::Error(e)));
				error += 1;
			}
		}
	}
	Statuses {
		link_tuples: statuses,
		missing,
		not_symlink,
		correct,
		incorrect,
		error,
	}
}

fn print_statuses(statuses: &Statuses) {
	let mut shorthand = Vec::<ColoredString>::new();
	let mut longhand = Vec::<String>::new();

	for (name, link, status) in statuses.link_tuples.iter() {
		match status {
			LinkStatus::Missing => {
				shorthand.push("M".yellow());
				longhand.push(format!(
					"{}: missing\n\n\t{} -> {}",
					name, link.from, link.to
				));
			}
			LinkStatus::NotSymlink => {
				shorthand.push("S".red());
				longhand.push(format!(
					"{}: is file/not symlink\n\n\t{} -> {}",
					name, link.from, link.to
				));
			}
			LinkStatus::Correct => {
				shorthand.push(".".green());
			}
			LinkStatus::Incorrect(actual_target) => {
				shorthand.push("I".red());
				longhand.push(format!(
					"{}: incorrect\n\n\t{} -> {}; (actual target: {})",
					name, link.from, link.to, actual_target
				));
			}
			LinkStatus::Error(e) => {
				shorthand.push("E".red());
				longhand.push(format!(
					"{}: error\n\n\t{} -> {}; error: {:#?}",
					name, link.from, link.to, e
				));
			}
		}
		longhand.push("".to_string());
	}
	for char in shorthand {
		print!("{}", char);
	}

	println!();

	for line in longhand {
		println!("{}", line);
	}
	println!();
	println!(
		"missing: {}, not symlink: {}, correct: {}, incorrect: {}, error: {}",
		statuses.missing.to_string().yellow(),
		statuses.not_symlink.to_string().red(),
		statuses.correct.to_string().green(),
		statuses.incorrect.to_string().red(),
		statuses.error.to_string().red()
	);
}

fn clean(map: &HashMap<String, Link>) {}

fn main() {
	let matches = Command::new("slmap")
		.about("symlink manager")
		.version("0.1.2")
		.arg_required_else_help(true)
		.author("Dustin Miller")
		.arg(
			Arg::new("command")
				.help("which command to run")
				.value_parser(["create", "update", "status", "clean"]),
		)
		.arg(
			Arg::new("map_file")
				.help("Map file location")
				.default_value("map.toml"),
		)
		.arg(
			Arg::new("lock_file")
				.help("Lock file location")
				.default_value("lock.toml"),
		)
		.get_matches();

	let command = matches.get_one::<String>("command").unwrap();
	let map_file_string = matches.get_one::<String>("map_file").unwrap();
	let lock_file_string = matches.get_one::<String>("lock_file").unwrap();
	// let mut map_file = File::open(map_file_string).unwrap();

	let mut map_file = OpenOptions::new()
		.read(true)
		.open(map_file_string)
		.expect("Unable to open file");

	let mut lock_file = OpenOptions::new()
		.read(true)
		.create(true)
		.write(true)
		.open(lock_file_string)
		.expect("unable to open lock file");

	println!(
		"command: {}, map: {}, lock: {}",
		command, map_file_string, lock_file_string
	);

	let map = read_map(&mut map_file);
	let lock_map = read_map(&mut lock_file);

	match command.as_str() {
		"create" => {
			let statuses = statuses(&map);
			if statuses.error + statuses.incorrect + statuses.not_symlink > 0 {
				println!("Refusing to create links, there are conflicts or errors");
				print_statuses(&statuses);
			} else if statuses.correct > 0 {
				println!("Refusing to create links, they are already correct");
				print_statuses(&statuses);
			} else if statuses.missing == 0 {
				println!("There is nothing to do; no writes will be made");
			} else {
				create(&map);
			}
		}
		"update" => update(&map, &lock_map),
		"status" => {
			// let statuses = statuses(&map);
			print_statuses(&statuses(&map));
		}
		"clean" => clean(&lock_map),
		_ => {
			panic!("Invalid command");
		}
	}
}

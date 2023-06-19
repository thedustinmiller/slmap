#![allow(unused_variables, unused_imports)]

use std::{
	collections::HashMap,
	fs::{self, File, OpenOptions},
	io::{Read, Write},
	os::unix,
	path::{Path, PathBuf},
};

use clap::{self, Arg, Command};
use colored::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Link {
	from: String,
	to: String,
	#[serde(default = "default_root")]
	root: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
enum LinkStatus {
	Correct,
	Incorrect(String),
	NotSymlink,
	Missing,
	Error(String),
}

fn default_root() -> bool {
	false
}

impl Link {
	fn resolved_path_to(&self) -> Result<PathBuf, String> {
		match resolve_path(self.to.clone()) {
			Ok(p) => Ok(p),
			Err(e) => Err(e),
		}
	}

	fn resolved_path_from(&self) -> Result<PathBuf, String> {
		match resolve_path(self.from.clone()) {
			Ok(p) => Ok(p),
			Err(e) => Err(e),
		}
	}

	fn check_link(&mut self) -> LinkStatus {
		check_link(self)
	}
}

fn resolve_path(path: String) -> Result<PathBuf, String> {
	match shellexpand::full(&path) {
		Ok(p) => Ok(PathBuf::from(p.into_owned())),
		Err(e) => Err(e.var_name),
	}
}

fn check_link(link: &Link) -> LinkStatus {
	let from_path = link.resolved_path_from().unwrap();
	let to_path = link.resolved_path_to().unwrap();

	if !to_path.exists() {
		LinkStatus::Missing
	} else if !to_path.is_symlink() {
		LinkStatus::NotSymlink
	} else {
		match to_path.read_link() {
			Ok(actual_target) => {
				if actual_target == from_path {
					LinkStatus::Correct
				} else {
					LinkStatus::Incorrect(
						actual_target
							.as_path()
							.to_str()
							.expect("Failed to convert path to string")
							.to_string(),
					)
				}
			}
			Err(e) => LinkStatus::Error(e.to_string()),
		}
	}
}

fn create_link(link: &Link) {
	let from_path = link.resolved_path_from().unwrap();
	let to_path = link.resolved_path_to().unwrap();

	if to_path.is_dir() {
		fs::create_dir_all(to_path.as_path()).expect("Failed to create directory");
	} else if to_path.is_file() {
		fs::create_dir_all(to_path.parent().unwrap()).expect("Failed to remove file");
	}

	unix::fs::symlink(from_path, to_path).expect("Failed to create symlink");
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
		create_link(link);
	}
}

fn update(map: &HashMap<String, Link>, lock_map: &HashMap<String, Link>) {}

fn status(map: &HashMap<String, Link>) -> Vec<(&String, &Link, LinkStatus)> {
	let mut statuses = Vec::new();
	for (name, link) in map {
		match check_link(link) {
			LinkStatus::Missing => {
				statuses.push((name, link, LinkStatus::Missing));
				// println!("{} -> {}: missing", link.from, link.to);
			}
			LinkStatus::NotSymlink => {
				statuses.push((name, link, LinkStatus::NotSymlink));
				// println!("{} -> {}: not symlink", link.from, link.to);
			}
			LinkStatus::Correct => {
				statuses.push((name, link, LinkStatus::Correct));
				// println!("{} -> {}: correct", link.from, link.to);
			}
			LinkStatus::Incorrect(actual_target) => {
				statuses.push((name, link, LinkStatus::Incorrect(actual_target)));
				// println!(
				// 	"{} -> {}: incorrect (actual target: {})",
				// 	link.from, link.to, actual_target
				// );
			}
			LinkStatus::Error(e) => {
				statuses.push((name, link, LinkStatus::Error(e)));
				// println!("{} -> {}: error: {:#?}", link.from, link.to, e);
			}
		}
	}
	statuses
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
			let statuses: Vec<LinkStatus> = status(&map)
				.into_iter()
				.map(|(name, link, status)| status)
				.collect();
			if statuses.iter().any(|status| *status != LinkStatus::Correct) {
				println!("Cannot create links, status is not correct");
			} else {
				// create(&map)
				println!("dry run");
			}
		}
		"update" => update(&map, &lock_map),
		"status" => {
			let statuses = status(&map);
			let mut shorthand = Vec::<ColoredString>::new();
			let mut longhand = Vec::<String>::new();
			let mut missing = 0;
			let mut not_symlink = 0;
			let mut correct = 0;
			let mut incorrect = 0;
			let mut error = 0;

			for (name, link, status) in statuses {
				match status {
					LinkStatus::Missing => {
						shorthand.push("M".yellow());
						missing += 1;
						longhand.push(format!("{}: missing\n\t{} -> {}", name, link.from, link.to));
					}
					LinkStatus::NotSymlink => {
						shorthand.push("S".red());
						not_symlink += 1;
						longhand.push(format!(
							"{}: not symlink\n\t{} -> {}",
							name, link.from, link.to
						));
					}
					LinkStatus::Correct => {
						shorthand.push(".".green());
						correct += 1;
					}
					LinkStatus::Incorrect(actual_target) => {
						shorthand.push("I".red());
						incorrect += 1;
						longhand.push(format!(
							"{}: incorrect\n\t{} -> {}; (actual target: {})",
							name, link.from, link.to, actual_target
						));
					}
					LinkStatus::Error(e) => {
						shorthand.push("E".red());
						error += 1;
						longhand.push(format!(
							"{}: error\n\t{} -> {}; error: {:#?}",
							name, link.from, link.to, e
						));
					}
				}
			}
			for char in shorthand {
				print!("{}", char);
			}

			println!();

			for line in longhand {
				println!("{}", line);
			}
			println!(
				"missing: {}, not symlink: {}, correct: {}, incorrect: {}, error: {}",
				missing.to_string().yellow(),
				not_symlink.to_string().red(),
				correct.to_string().green(),
				incorrect.to_string().red(),
				error.to_string().red()
			);
		}
		"clean" => clean(&lock_map),
		_ => {
			panic!("Invalid command");
		}
	}
}

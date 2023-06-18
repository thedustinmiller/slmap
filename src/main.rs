#![feature(absolute_path)]

use std::{
	collections::HashMap,
	env,
	fs::{self, File, OpenOptions},
	io::{Read, Seek, Write},
	path::{self, Path, PathBuf},
};

use clap::{self, Arg, Command};
use serde::{Deserialize, Serialize};
use shellexpand::LookupError;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Link {
	from: String,
	to: String,
	#[serde(default = "default_root")]
	root: bool,
}

#[derive(Debug)]
enum LinkStatus {
	Correct,
	Incorrect(String),
	NotSymlink,
	Missing,
	Error(std::io::Error),
}

fn default_root() -> bool {
	false
}

fn resolve_path(path: String) -> Result<PathBuf, LookupError<VarError>> {
	match shellexpand::full(path.into()){
		Ok(p) => p.into(),
		Err(e) => Err(e),
	}
}

fn create_link(link: &Link) {
	fs::create_dir_all(str_to_abs(&link.to).as_path().parent().unwrap())
		.expect("Failed to create directory");

	// match link.link_type.as_str() {
	//     "soft" => {
	//         std::os::unix::fs::symlink(str_to_abs(&link.from), str_to_abs(&link.to))
	//             .expect("Failed to create symlink");
	//     }
	//     "hard" => {
	//         //std::os::unix::fs::hard_link(link.from, link.to);
	//         panic!("Hard links are not supported yet");
	//     }
	//     _ => {
	//         panic!("Invalid link type");
	//     }
	// }
}

fn check_link(link: &Link) -> LinkStatus {
	let from_path = Path::new(&link.from);
	let to_path = Path::new(&link.to);

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
			Err(e) => LinkStatus::Error(e),
		}
	}
}

fn read_map(map_file: &mut File) -> HashMap<String, Link> {
	let mut file_string = String::new();
	map_file
		.read_to_string(&mut file_string)
		.expect("read fail");

	let map: HashMap<String, Link> = toml::from_str(&file_string).unwrap();

	map
}

// fn purge_links(lock_file: &mut File) {
//     let mut file_string = String::new();
//     lock_file
//         .read_to_string(&mut file_string)
//         .expect("read fail");
//
//     let lock: HashMap<String, Link> = toml::from_str(&file_string).unwrap();
//
//     for (_name, link) in &lock {
//         destroy_link(link);
//     }
//     lock_file.set_len(0).expect("erase failed");
//     lock_file.rewind().expect("rewind fail");
//     lock_file.sync_all().expect("sync fail");
// }

fn main() {
	let matches = Command::new("slmap")
		.about("symlink manager")
		.version("0.1.2")
		.arg_required_else_help(true)
		.author("Dustin Miller")
		.arg(
			Arg::new("command")
				.help("which command to run")
				.value_parser(["read", "status", "clean"]),
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

	let map_file_string = matches.value_of("map_file").unwrap();
	let lock_file_string = matches.value_of("lock_file").unwrap();

	let mut map_file = File::open(map_file_string).unwrap();

	println!("command: {}", matches.value_of("command").unwrap());
	println!("map file: {}", map_file_string);
	println!("lock file: {}", lock_file_string);

	let from_tilde = shellexpand::tilde("~/demo.txt").to_string();
	let to_tilde = shellexpand::tilde("demo.txt").to_string();

	println!("tilde from: {}", from_tilde);
	println!("tilde to: {}", to_tilde);

	let from_full = shellexpand::full("$RUSTUP_HOME/thing.txt").unwrap();
	let to_full = shellexpand::full("~/$PAGER").unwrap();

	println!("full from: {}", from_full);
	println!("full to: {}", to_full);

	// for (key, value) in env::vars() {
	// 	println!("{key}: {value}");
	// }

	let map = read_map(&mut map_file);

	for (name, link) in &map {
		println!("{} -> {}: {:#?}", link.from, link.to, check_link(link));
	}

	// println!("{:#?}", from.canonicalize().unwrap());
	//
	// std::os::unix::fs::symlink(from, to).expect("Failed to create symlink");

	// println!("{:#?}",
	// 		 Path::new("sample/map.toml").exists()
	// );

	// println!("{:#?}",
	// 		 match Path::new("sample/map.toml").read_link() {
	// 			 Ok(path) => path,
	// 			 Err(e) => panic!("error: {:#?}", e),
	// 		 }
	// );

	// read_map(&mut map_file);

	// println!("{}", Path::new("~/Desktop/tools/").parent().unwrap().to_str().unwrap());

	// let mut map_file = OpenOptions::new()
	//     .read(true)
	//     .open(map_file_string)
	//     .expect("Unable to open file");
	//
	// let mut lock_file = OpenOptions::new()
	//     .read(true)
	//     .create(true)
	//     .write(true)
	//     .append(true)
	//     .open(lock_file_string)
	//     .expect("unable to open lock file");

	// match matches.value_of("command").unwrap() {
	//     "read" => {
	//         read_map(&mut map_file, &mut lock_file);
	//     }
	//     "clean" => {
	//         purge_links(&mut lock_file);
	//     }
	//     _ => {
	//         panic!("Invalid command");
	//     }
	// }
}

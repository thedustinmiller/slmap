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
	pub from: String,
	pub to: String,
	#[serde(default = "default_root")]
	pub root: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub enum LinkStatus {
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
	pub fn resolved_path_to(&self) -> Result<PathBuf, String> {
		match resolve_path(self.to.clone()) {
			Ok(p) => Ok(p),
			Err(e) => Err(e),
		}
	}

	pub fn resolved_path_from(&self) -> Result<PathBuf, String> {
		match resolve_path(self.from.clone()) {
			Ok(p) => Ok(p),
			Err(e) => Err(e),
		}
	}

	pub fn check_link(&self) -> LinkStatus {
		let from_path = self.resolved_path_from().unwrap();
		let to_path = self.resolved_path_to().unwrap();

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

	pub fn create_link(&self) {
		let from_path = self.resolved_path_from().unwrap();
		let to_path = self.resolved_path_to().unwrap();

		if to_path.is_dir() {
			fs::create_dir_all(to_path.as_path()).expect("Failed to create directory");
		} else if to_path.is_file() {
			fs::create_dir_all(to_path.parent().unwrap()).expect("Failed to remove file");
		}

		unix::fs::symlink(from_path, to_path).expect("Failed to create symlink");
	}
}

fn resolve_path(path: String) -> Result<PathBuf, String> {
	match shellexpand::full(&path) {
		Ok(p) => Ok(PathBuf::from(p.into_owned())),
		Err(e) => Err(e.var_name),
	}
}

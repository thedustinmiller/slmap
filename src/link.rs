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

	pub fn check_link(&self) -> LinkStatus {
		let target = self.resolved_target().unwrap();
		let link_name = self.resolved_link_name().unwrap();

		if !link_name.exists() {
			LinkStatus::Missing
		} else if !link_name.is_symlink() {
			LinkStatus::NotSymlink
		} else {
			match link_name.read_link() {
				Ok(actual_target) => {
					if actual_target == target {
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
		let target = self.resolved_target().unwrap();
		let link_name = self.resolved_link_name().unwrap();

		if link_name.is_dir() {
			fs::create_dir_all(link_name.as_path()).expect("Failed to create directory");
		} else if link_name.is_file() {
			fs::create_dir_all(link_name.parent().unwrap()).expect("Failed to remove file");
		}

		unix::fs::symlink(target, link_name).expect("Failed to create symlink");
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

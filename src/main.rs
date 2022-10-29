#![feature(absolute_path)]

use clap::{self, Arg, Command};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap},
    fs::{self, File, OpenOptions},
    io::{Read, Seek, Write},
    path::{self, PathBuf},
};

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Link {
    // [filename]
    // from = path/to/file
    // to = path/to/file
    // type = soft/hard (default: soft)
    // perms = 0644 (default: 0644)
    // owner = user (default: current user)
    // group = group (default: current group)
    from: String,
    to: String,
    #[serde(default = "default_type")]
    link_type: String,
    #[serde(default = "default_owner")]
    owner: String,
    #[serde(default = "default_group")]
    group: String,
    #[serde(default = "default_destroy")]
    destroy: bool,
}

#[allow(dead_code)]
#[derive(Hash, Eq, PartialEq)]
enum LinkStatus {
    Create,
    Update,
    Destroy,
    Nothing,
}

fn default_type() -> String {
    "soft".to_string()
}
fn default_owner() -> String {
    "".to_string()
}
fn default_group() -> String {
    "".to_string()
}
fn default_destroy() -> bool {
    false
}

fn str_to_abs(path: &String) -> PathBuf {
    path::absolute(path.as_str()).unwrap().to_path_buf()
}

fn create_link(link: &Link) {
    fs::create_dir_all(str_to_abs(&link.to).as_path().parent().unwrap())
        .expect("Failed to create directory");

    match link.link_type.as_str() {
        "soft" => {
            std::os::unix::fs::symlink(str_to_abs(&link.from), str_to_abs(&link.to))
                .expect("Failed to create symlink");
        }
        "hard" => {
            //std::os::unix::fs::hard_link(link.from, link.to);
            panic!("Hard links are not supported yet");
        }
        _ => {
            panic!("Invalid link type");
        }
    }
}

fn destroy_link(link: &Link) {
    match fs::remove_file(str_to_abs(&link.to).as_path()) {
        Ok(_) => {}
        Err(e) => {
            println!("Failed to destroy link: {}", e);
        }
    }
}

fn check_link(link: &Link) -> LinkStatus {
    let path = str_to_abs(&link.to);

    if path.exists() {
        if path.is_symlink() {
            let link_path = fs::read_link(path).unwrap();
            if link_path.to_str().expect("msg") == str_to_abs(&link.from).to_str().expect("msg") {
                // return LinkStatus::Nothing;
                return LinkStatus::Update;
            } else {
                return LinkStatus::Update;
            }
        } else {
            panic!("not a symlink?");
        }
    } else {
        return LinkStatus::Create;
    }
}

fn update_lock(name: &String, link: &Link, file: &mut File) {
    let mut s = String::new();
    file.rewind().expect("rewind fail");
    file.read_to_string(&mut s).expect("read fail");

    let mut table: HashMap<String, Link> = toml::from_str(&s).unwrap();
    table.remove(name);
    table.insert(name.clone(), link.clone());

    file.set_len(0).expect("erase failed");
    file.rewind().expect("rewind fail");

    let s = toml::to_string(&table).unwrap().as_bytes().to_owned();
    file.write_all(&s).expect("write fail");
    file.sync_all().expect("sync fail");
}

fn read_map(map_file: &mut File, lock_file: &mut File) {
    let mut file_string = String::new();
    map_file
        .read_to_string(&mut file_string)
        .expect("read fail");

    let file_string = fs::read_to_string("map.toml").expect("Unable to read file");

    let map: HashMap<String, Link> = toml::from_str(&file_string).unwrap();

    for (name, link) in &map {
        println!("{:#?}", name);
        println!("{:#?}", link);

        match check_link(link) {
            LinkStatus::Create => {
                println!("Create");
                create_link(link);
                update_lock(name, link, lock_file);
            }
            LinkStatus::Update => {
                println!("update");
                destroy_link(link);
                create_link(link);
                update_lock(name, link, lock_file)
            }
            LinkStatus::Destroy => {
                println!("destroy");
                destroy_link(link);
            }
            LinkStatus::Nothing => {
                println!("nothing");
            }
        }
    }
}

fn purge_links(lock_file: &mut File) {
    let mut file_string = String::new();
    lock_file
        .read_to_string(&mut file_string)
        .expect("read fail");

    let lock: HashMap<String, Link> = toml::from_str(&file_string).unwrap();

    for (_name, link) in &lock {
        destroy_link(link);
    }
    lock_file.set_len(0).expect("erase failed");
    lock_file.rewind().expect("rewind fail");
    lock_file.sync_all().expect("sync fail");
}

fn main() {
    let matches = Command::new("slmap")
        .about("symlink manager")
        .version("0.1.1")
        .arg_required_else_help(true)
        .author("Dustin Miller")
        .arg(
            Arg::new("command")
                .help("which command to run")
                .value_parser(["read", "clean"]),
        )
        .arg(
            Arg::new("map_file")
                .short('m')
                .long("map")
                .help("Map file location")
                .default_value("map.toml")
                .takes_value(true),
        )
        .arg(
            Arg::new("lock_file")
                .short('l')
                .long("lock")
                .help("Lock file location")
                .default_value("lock.toml")
                .takes_value(true),
        )
        .get_matches();

    let map_file_string = matches.value_of("map_file").unwrap();
    let lock_file_string = matches.value_of("lock_file").unwrap();

    let mut map_file = OpenOptions::new()
        .read(true)
        .open(map_file_string)
        .expect("Unable to open file");

    let mut lock_file = OpenOptions::new()
        .read(true)
        .create(true)
        .write(true)
        .append(true)
        .open(lock_file_string)
        .expect("unable to open lock file");

    match matches.value_of("command").unwrap() {
        "read" => {
            read_map(&mut map_file, &mut lock_file);
        }
        "clean" => {
            purge_links(&mut lock_file);
        }
        _ => {
            panic!("Invalid command");
        }
    }
}

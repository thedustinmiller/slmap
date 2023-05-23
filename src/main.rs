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
    // root = false
    from: String,
    to: String,
    #[serde(default = "default_root")]
    root: bool,
}

fn default_root() -> bool {
    false
}

fn str_to_abs(path: &String) -> PathBuf {
    path::absolute(path.as_str()).unwrap().to_path_buf()
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


// fn check_link(link: &Link) -> LinkStatus {
//     let path = str_to_abs(&link.to);
//
//     if path.exists() {
//         if path.is_symlink() {
//             let link_path = fs::read_link(path).unwrap();
//             if link_path.to_str().expect("msg") == str_to_abs(&link.from).to_str().expect("msg") {
//                 // return LinkStatus::Nothing;
//                 return LinkStatus::Update;
//             } else {
//                 return LinkStatus::Update;
//             }
//         } else {
//             return LinkStatus::Update; //is a file apparently
//         }
//     } else {
//         return LinkStatus::Create;
//     }
// }


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
    }
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
                .value_parser(["read", "clean"]),
        )
        .arg(
            Arg::new("map_file")
                .help("Map file location")
                .default_value("map.toml")
        )
        .arg(
            Arg::new("lock_file")
                .help("Lock file location")
                .default_value("lock.toml")
        )
        .get_matches();

    let map_file_string = matches.value_of("map_file").unwrap();
    let lock_file_string = matches.value_of("lock_file").unwrap();

    println!("map file: {}", map_file_string);
    println!("lock file: {}", lock_file_string);

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

#![feature(absolute_path)]

use serde::{Deserialize, Serialize};
use std::{fs, collections::HashMap, path::{self, Path, PathBuf}};
use whoami;

#[derive(Serialize, Deserialize, Debug)]
struct Link{
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
    group: String
}

fn default_type() -> String {
    "soft".to_string()
}

fn default_owner() -> String {
    whoami::username()
}

fn default_group() -> String {
    whoami::username()
}

fn str_to_abs(path: &String) -> PathBuf{
    path::absolute(path.as_str()).unwrap().to_path_buf()
}

fn create_link(link: &Link) {
    println!("{:#?}", str_to_abs(&link.to).as_path().parent().unwrap());
    fs::create_dir_all(str_to_abs(&link.to).as_path().parent().unwrap()).expect("Failed to create directory");

    match link.link_type.as_str(){
        "soft" => {
            std::os::unix::fs::symlink(str_to_abs(&link.from), str_to_abs(&link.to)).expect("Failed to create symlink");
        },
        "hard" => {
            //std::os::unix::fs::hard_link(link.from, link.to);
            panic!("Hard links are not supported yet");
        },
        _ => {
            panic!("Invalid link type");
        }
    }
}


fn main() {


    let link = Link {
        from: "/tmp/test".to_string(),
        to: "/tmp/test".to_string(),
        link_type: "soft".to_string(),
        owner: "root".to_string(),
        group: "root".to_string()
    };

    let toml = toml::to_string(&link).unwrap();
    println!("{:#?}", toml);


    let file_string = fs::read_to_string("map.toml").expect("Unable to read file");

    let map: HashMap<String, Link> = toml::from_str(&file_string).unwrap();

    for (name, link) in &map {
        println!("{:#?}", name);
        println!("{:#?}", link);
        create_link(&link);
    }

    println!("{:#?}", map);

}

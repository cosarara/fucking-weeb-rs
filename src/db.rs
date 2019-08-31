// Fucking Weeb
// Copyright © Jaume Delclòs Coll
//
// This file is part of Fucking Weeb.
//
// Fucking Weeb is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// Fucking Weeb is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with Fucking Weeb.  If not, see <http://www.gnu.org/licenses/>.

use std::fs::File;
use std::io::prelude::*;
use serde_json;
use dirs;
use std::fs;

#[derive(Serialize, Deserialize, Clone)]
pub struct Show {
    pub name: String,
    pub path: String,
    pub poster_path: String,
    pub current_ep: i32,
    pub total_eps: i32,
    #[serde(default)]
    pub regex: String,
    #[serde(default)]
    pub player: String,
}

impl Default for Show {
    fn default() -> Show {
        Show {
            name: "".to_string(),
            path: "".to_string(),
            poster_path: "".to_string(),
            current_ep: 1,
            total_eps: 24,
            regex: "".to_string(),
            player: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Settings {
    pub player: String,
    pub path: String,
    pub autoplay: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WeebDB {
    pub settings: Settings,
    pub shows: Vec<Show>,
}

pub fn load_db() -> WeebDB {
    let default_settings = WeebDB {
        settings: Settings {
            player: "mpv".to_string(),
            path: "".to_string(),
            autoplay: false,
        },
        shows: vec![
            Show {
                name: "Ranma".to_string(),
                path: "/home/jaume/videos/series/1-More/Ranma/".to_string(),
                poster_path: "/home/jaume/.local/share/fucking-weeb/ranma.jpg".to_string(),
                current_ep: 25,
                total_eps: 150,
                regex: " 0*{}[^0-9]".to_string(),
                player: "".to_string(),
            },
            Show {
                name: "Neon Genesis Evangelion".to_string(),
                path: "/home/jaume/videos/series/0-Sorted/neon_genesis_evangelion-1080p-renewal_cat/".to_string(),
                poster_path: "/home/jaume/.local/share/fucking-weeb/Neon%20Genesis%20Evangelion.jpg".to_string(),
                current_ep: 5,
                total_eps: 26,
                regex: "".to_string(),
                player: "".to_string(),
            }
        ],
    };

    let config_path = dirs::dirs().config_dir().join("fw-rs-db.json");
    if !config_path.is_file() {
        println!("db file not found");
        return default_settings;
    }

    let mut file = match File::open(config_path) {
        Err(e) => {
            println!("error opening db file: {}", e.to_string());
            return default_settings;
        },
        Ok(file) => file
    };
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(e) => {
            println!("error reading db: {}", e.to_string());
            return default_settings;
        }
        Ok(_) => ()
    }
    match serde_json::from_str(&s) {
        Ok(a) => a,
        Err(e) => {
            println!("error decoding db json: {}", e.to_string());
            default_settings
        }
    }
}

pub fn save_db(settings: &Settings, items: &Vec<Show>) {
    let db = WeebDB {
        settings: settings.clone(),
        shows: items.clone(),
    };
    // TODO: rotate file for safety
    // what happens if the process is killed mid-write?
    //let encoded = rsjson::as_pretty_json(&db);
    let encoded = serde_json::to_string(&db).unwrap();
    //println!("{}", encoded);
    // TODO: handle errors
    let config_path = dirs::dirs().config_dir();
    fs::create_dir_all(config_path)
        .expect("cannot create configuration directory");
    let config_path = config_path.join("fw-rs-db.json");
    let mut file = File::create(config_path).expect("cannot create db file");
    match file.write_all(format!("{}\n", encoded).as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error saving db: {}", e);
        }
    };
}



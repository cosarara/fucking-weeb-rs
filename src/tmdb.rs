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

use json;
use xdg;
use regex::Regex;
use std::fs::File;
use std::io::prelude::*;

pub static TMDB: &'static str = "https://api.themoviedb.org/3/";
pub static TMDB_KEY: &'static str = "api_key=fd7b3b3e7939e8eb7c8e26836b8ea410";

lazy_static! {
    pub static ref TMDB_BASE_URL: Result<String, String> = get_tmdb_base_url();
}

pub fn json_get(url: &str) -> Result<json::JsonValue, String> {
    let data = crate::soup::get_sync(&url)
        .map_err(|e| format!("error downloading json: {}", e))?;
    let text = std::str::from_utf8(&data).map_err(|e| format!("error decoding json text: {}", e))?;
    json::parse(&text).map_err(|e| e.to_string())
}

fn get_tmdb_base_url() -> Result<String, String> {
    let url = format!("{}configuration?{}", TMDB, TMDB_KEY);
    let parsed = json_get(&url)?;
    let ref json_tmdb_base_url = parsed["images"]["base_url"];
    json_tmdb_base_url.as_str()
        .map(|x| x.to_string())
        .ok_or("base_url string not found in json".to_string())
}

pub fn download_image(image_url: &str) -> Result<String, String> {
    println!("starting download");
    //let image_file = https_get_bin(&image_url)
    let image_file = crate::soup::get_sync(&image_url)
        .map_err(|e| format!("error downloading image: {}", e))?;
    println!("finished download");

    let file_name = Regex::new(r".*/").unwrap().
        replace(&image_url, "").into_owned();

    // todo check this unwrap
    let xdg_dirs = xdg::BaseDirectories::with_prefix("fucking-weeb").unwrap();
    let path = xdg_dirs.place_data_file(file_name.clone())
        .map_err(|_| "cannot create data directory")?;

    let mut file = File::create(path.clone())
        .map_err(|e| format!("error opening image file for writing: {}", e))?;
    file.write_all(&image_file)
        .map_err(|e| format!("error saving image: {}", e))?;
    return Ok(path.to_string_lossy().to_string());
}


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

use hyper;
use json;
use xdg;
use regex::Regex;
use hyper_native_tls::{NativeTlsClient, native_tls};
use std::fs::File;
use std::io::prelude::*;

pub static TMDB: &'static str = "https://api.themoviedb.org/3/";
pub static TMDB_KEY: &'static str = "api_key=fd7b3b3e7939e8eb7c8e26836b8ea410";

lazy_static! {
    pub static ref TMDB_BASE_URL: Result<String, String> = get_tmdb_base_url();
}

fn make_https_client() -> native_tls::Result<hyper::client::Client> {
    NativeTlsClient::new().map(
        |ssl| {
            let connector = hyper::net::HttpsConnector::new(ssl);
            let client = hyper::client::Client::with_connector(connector);
            client
        })
}

fn https_get(url: &str) -> Result<String, String> {
    let client = match make_https_client() {
        Ok(ssl) => ssl,
        Err(e) => {
            return Err(format!("error creating https client: {}", e));
        }
    };
    let req = client.get(url);
    let mut res = match req.send() {
        Ok(res) => res,
        Err(e) => {
            return Err(format!("error making request: {}", e));
        }
    };
    let mut text = String::new();
    match res.read_to_string(&mut text) {
        Ok(_) => (),
        Err(e) => {
            return Err(format!("error reading response: {}", e));
        }
    }
    Ok(text)
}

pub fn json_get(url: &str) -> Result<json::JsonValue, String> {
    let text = match https_get(url) {
        Ok(text) => text,
        Err(e) => {
            return Err(e.to_string());
        }
    };
    let parsed = match json::parse(&text) {
        Ok(o) => o,
        Err(e) => {
            return Err(e.to_string());
        },
    };
    Ok(parsed)
}

fn get_tmdb_base_url() -> Result<String, String> {
    let url = format!("{}configuration?{}", TMDB, TMDB_KEY);
    let parsed = match json_get(&url) {
        Ok(text) => text,
        Err(e) => {
            return Err(e);
        }
    };
    let ref json_tmdb_base_url = parsed["images"]["base_url"];
    match json_tmdb_base_url.as_str().map(|x| x.to_string()) {
        Some(a) => Ok(a),
        None => Err("base_url string not found in json".to_string()),
    }
}

fn https_get_bin(url: &str) -> Result<Vec<u8>, String> {
    let client = match make_https_client() {
        Ok(ssl) => ssl,
        Err(e) => {
            return Err(format!("error creating https client: {}", e));
        }
    };
    let req = client.get(url);
    let mut res = match req.send() {
        Ok(res) => res,
        Err(e) => {
            return Err(format!("error making request: {}", e));
        }
    };
    let mut file = Vec::<u8>::new();
    match res.read_to_end(&mut file) {
        Ok(_) => (),
        Err(e) => {
            return Err(format!("error reading response: {}", e));
        }
    }
    Ok(file)
}

pub fn download_image(image_url: &str) -> Result<String, String> {
    let image_file = match https_get_bin(&image_url) {
        Ok(a) => a,
        Err(e) => {
            return Err(format!("error downloading image: {}", e));
        }
    };
    let file_name = Regex::new(r".*/").unwrap().
        replace(&image_url, "").into_owned();

    let xdg_dirs = xdg::BaseDirectories::with_prefix("fucking-weeb").unwrap();
    let path = xdg_dirs.place_data_file(file_name.clone())
        .expect("cannot create data directory");

    let mut file = match File::create(path.clone()) {
        Ok(f) => f,
        Err(e) => {
            return Err(format!("error opening image file for writing: {}", e));
        }
    };
    match file.write_all(&image_file) {
        Ok(_) => (),
        Err(e) => {
            return Err(format!("error saving image: {}", e));
        }
    }
    return Ok(path.to_str().unwrap().to_string());
}


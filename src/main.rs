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

extern crate gtk;
extern crate gdk_sys;
extern crate gdk_pixbuf;
extern crate rustc_serialize;
extern crate regex;
extern crate hyper;
extern crate hyper_native_tls;
extern crate xdg;

// yes we have 2 different jsons k
#[macro_use]
extern crate json;

#[macro_use]
extern crate lazy_static;

use rustc_serialize::json as rsjson;

use gtk::prelude::*;

use gtk::{Button, Window, WindowType, Box, Orientation,
    Label, IconSize, SearchEntry, ScrolledWindow, Viewport,
    FlowBox, SelectionMode, EventBox, Image};

use gdk_pixbuf::Pixbuf;

use std::io::prelude::*;
use std::fs;
use std::fs::File;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use regex::Regex;

use hyper_native_tls::{NativeTlsClient, native_tls};

#[derive(RustcDecodable, RustcEncodable, Clone)]
struct Show {
    name: String,
    path: String,
    poster_path: String,
    current_ep: i32,
    total_eps: i32,
    regex: String,
    player: String,
}

#[derive(RustcDecodable, RustcEncodable, Clone)]
struct Settings {
    player: String,
    path: String,
    autoplay: bool,
}

#[derive(RustcDecodable, RustcEncodable, Clone)]
struct WeebDB {
    settings: Settings,
    shows: Vec<Show>,
}

static APP_TITLE: &'static str = "Fucking Weeb";
static TMDB: &'static str = "https://api.themoviedb.org/3/";
static TMDB_KEY: &'static str = "api_key=fd7b3b3e7939e8eb7c8e26836b8ea410";

lazy_static! {
    static ref TMDB_BASE_URL: Result<String, String> = get_tmdb_base_url();
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

fn json_get(url: &str) -> Result<json::JsonValue, String> {
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

fn find_ep(dir: &str, num: u32, regex: &str) -> Result<PathBuf, String> {
    let dir = std::path::Path::new(dir);
    if !dir.is_dir() {
        return Err("not a directory".to_string());
    }
    // getting the iterator can error
    let files = match fs::read_dir(dir) {
        Ok(files) => files,
        Err(e) => return Err(e.to_string())
    };
    // each iteration can error too
    let files: Result<Vec<_>, _> = files.collect();
    let files = match files {
        Ok(files) => files,
        Err(e) => return Err(e.to_string())
    };
    println!("len: {}", files.len());
    if files.len() == 0 {
        return Err("nothing found".to_string());
    }
    let mut files: Vec<PathBuf> = files.iter().map(|x| x.path()).collect();
    files.sort();

    let regexes = if regex != "" {
        vec![regex]
    } else {
        vec![
            "(e|ep|episode|第)[0 ]*{}[^0-9]",
            "( |_|-|#|\\.)[0 ]*{}[^0-9]",
            "(^|[^0-9])[0 ]*{}[^0-9]",
            "{}[^0-9]"]
    };

    for r in regexes.iter().map(|x| x.replace("{}", &format!("{}", num))) {
        println!("{}", r);
        let re = Regex::new(&r).unwrap();
        let mut best_match: Option<&Path> = None;
        let mut best_score = 0; // lower is better
        for file in files.iter().map(|x| x.as_path()) {
            let file_name = file.to_str().unwrap();
            match re.find(file_name) {
                Some(mat) => {
                    //println!("{} matches at {}!", file_name, mat.start());
                    let score = mat.start();
                    if best_match.is_none() || score < best_score {
                        best_match = Some(file);
                        best_score = score;
                    }
                },
                None => ()
            };
        }
        match best_match {
            Some(path) => {
                //println!("best match {} at position {}!", path.to_str().unwrap(), best_score);
                return Ok(path.to_path_buf());
            },
            None => ()
        };
    }
    Err("matching file not found".to_string())
}

fn watch(show: &Show, player: &str) {
    let ref dir = show.path;
    let ref reg = show.regex;
    let ep = show.current_ep.abs() as u32;
    let path = match find_ep(&dir, ep, &reg) {
        Err(e) => {
            println!("{}", e); // TODO: gtk dialog
            return;
        },
        Ok(path) => path
    };
    println!("{}", path.to_str().unwrap());
    let player_full_cmd = if show.player == "" { player } else { &show.player };
    let mut cmd_parts = player_full_cmd.split(' ');
    let player_cmd = match cmd_parts.next() {
        Some(a) => a,
        None => "mpv"
    };
    let player_args: Vec<&str> = cmd_parts.collect();
    let cmd = Command::new(player_cmd)
        .args(&player_args)
        .arg(path.to_str().unwrap())
        .spawn();
    match cmd {
        Err(e) => {
            println!("{}", e); // TODO: gtk dialog
        },
        Ok(_) => ()
    }
}

fn make_title_label(text: &str) -> Label {
    let label = Label::new(None);
    label.set_markup(&format!("<span weight='bold' size='xx-large'>{}</span>", text));
    label
}

fn view_screen(window: &Window, items: &Vec<Show>, i: usize, settings: &Settings) {
    let show = items[i].clone();
    if let Some(child) = window.get_child() {
        child.destroy();
    };
    let main_box = Box::new(Orientation::Vertical, 0);
    main_box.set_spacing(10);
    main_box.set_margin_top(20);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_bottom(20);
    window.add(&main_box);

    // HEADER
    let title_box = Box::new(Orientation::Horizontal, 0);
    let title_label = make_title_label(&show.name);
    title_label.set_selectable(true);
    title_box.set_center_widget(Some(&title_label));
    main_box.pack_start(&title_box, false, true, 5);

    // rm button
    let remove_button = Button::new_from_icon_name(
        "gtk-remove", IconSize::Button.into());
    title_box.pack_end(&remove_button, false, true, 3);

    // edit button
    let edit_button = Button::new_from_icon_name(
        "gtk-edit", IconSize::Button.into());
    title_box.pack_end(&edit_button, false, true, 0);

    // cover
    let cover_event_box = EventBox::new();
    cover_event_box.set_events(
        gdk_sys::GDK_BUTTON_PRESS_MASK.bits() as i32);


    // TODO: use cairo here, to have it resize automagically
    let image = match Pixbuf::new_from_file_at_size(
        &show.poster_path, 300, 300) {
        Ok(pixbuf) => Image::new_from_pixbuf(Some(&pixbuf)),
        Err(_) => Image::new_from_icon_name("gtk-missing-image", 1)
    };

    cover_event_box.add(&image);

    main_box.pack_start(&cover_event_box, true, true, 5);

    // progress
    let progress_box = Box::new(Orientation::Horizontal, 0);
    main_box.pack_start(&progress_box, true, false, 5);
    let current_ep_adj = gtk::Adjustment::new(
        show.current_ep as f64, 1.0, show.total_eps as f64,
        1.0, 0.0, 0.0);
    let spin = gtk::SpinButton::new(Some(&current_ep_adj), 0.0, 0);
    progress_box.pack_start(&spin, false, true, 5);
    let total_label_text = format!("/ {}", show.total_eps);
    let total_label = Label::new(Some(&total_label_text));
    progress_box.pack_start(&total_label, false, true, 5);
    progress_box.set_halign(gtk::Align::Center);

    // watch button
    let watch_button = Button::new_with_label("Watch");
    main_box.pack_start(&watch_button, false, true, 2);

    // watch next button
    let watch_next_button = Button::new_with_label("Watch Next");
    main_box.pack_start(&watch_next_button, false, true, 2);

    // back
    let back_button = Button::new_with_label("Back");
    back_button.set_margin_top(20);
    main_box.pack_end(&back_button, false, true, 2);

    // connections
    let bw = window.clone();
    let bs = items.clone();
    let bspin = spin.clone();
    let bset = settings.clone();
    back_button.connect_clicked(move |_| {
        let mut items = bs.clone();
        let ep = bspin.get_value_as_int();
        items[i].current_ep = ep;
        main_screen(&bw, &items, &bset);
    });

    let dw = window.clone();
    let mut ds = items.clone();
    let dset = settings.clone();
    ds.remove(i);
    // FIXME: prompt confirmation
    remove_button.connect_clicked(move |_| {
        save_db(&dset, &ds);
        main_screen(&dw, &ds, &dset);
    });

    let ew = window.clone();
    let es = items.clone();
    let espin = spin.clone();
    let eset = settings.clone();
    edit_button.connect_clicked(move |_| {
        let mut items = es.clone();
        let ep = espin.get_value_as_int();
        items[i].current_ep = ep;
        edit_screen(&ew, &items, Some(i), &eset);
    });

    let wspin = spin.clone();
    let wshow = show.clone();
    let wplayer = settings.player.clone();
    watch_button.connect_clicked(move |_| {
        let ep = wspin.get_value_as_int();
        let mut show = wshow.clone();
        show.current_ep = ep;
        watch(&show, &wplayer);
    });

    let wnspin = spin.clone();
    let wnshow = show.clone();
    let wnplayer = settings.player.clone();
    watch_next_button.connect_clicked(move |_| {
        let mut show = wnshow.clone();
        let ep = wnspin.get_value_as_int();
        let max_ep = show.total_eps;
        let ep = if ep < max_ep { ep+1 } else { ep };
        wnspin.set_value(ep as f64);
        show.current_ep = ep;
        watch(&show, &wnplayer);
    });

    let vcspin = spin.clone();
    let vcitems = items.clone();
    let vcset = settings.clone();
    spin.connect_value_changed(move |_| {
        let ep = vcspin.get_value_as_int();
        //println!("{}", ep);
        let mut items = vcitems.clone();
        items[i].current_ep = ep;
        save_db(&vcset, &items);
    });

    window.show_all();
}

fn edit_screen(window: &Window, items: &Vec<Show>, i: Option<usize>,
               settings: &Settings) {
    let orig_items = items.clone();
    let mut adding = false;
    let mut items = items.clone();
    let show = match i {
        Some(i) => items[i].clone(),
        None => {
            let s = Show {
                name: "".to_string(),
                path: settings.path.clone(),
                poster_path: "".to_string(),
                current_ep: 1,
                total_eps: 24,
                regex: "".to_string(),
                player: "".to_string(),
            };
            items.push(s.clone());
            adding = true;
            s
        },
    };
    let i = match i {
        Some(i) => i,
        None => items.len() - 1
    };
    if let Some(child) = window.get_child() {
        child.destroy();
    };
    let main_box = Box::new(Orientation::Vertical, 0);
    main_box.set_spacing(20);
    main_box.set_margin_top(20);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_bottom(20);
    window.add(&main_box);

    let form = gtk::Grid::new();
    form.set_column_spacing(20);
    form.set_row_spacing(10);
    main_box.pack_start(&form, true, true, 5);

    let name_label = Label::new(Some("Name:"));
    name_label.set_xalign(1.0);
    let name_entry = gtk::Entry::new();
    name_entry.set_text(&show.name);
    name_entry.set_hexpand(true);
    form.attach(&name_label, 0, 0, 1, 1);
    form.attach(&name_entry, 1, 0, 3, 1);

    let path_label = Label::new(Some("Path:"));
    path_label.set_xalign(1.0);

    let path_picker = gtk::FileChooserButton::new(
        "Select the search path",
        gtk::FileChooserAction::SelectFolder);

    path_picker.set_filename(show.path);
    path_picker.set_hexpand(true);

    form.attach(&path_label, 0, 1, 1, 1);
    form.attach(&path_picker, 1, 1, 3, 1);

    let poster_label = Label::new(Some("Poster Image Path:"));
    poster_label.set_xalign(1.0);

    let poster_picker = gtk::FileChooserButton::new(
        "Select the poster image",
        gtk::FileChooserAction::Open);
    poster_picker.set_hexpand(true);
    poster_picker.set_filename(show.poster_path);

    let image_filter = gtk::FileFilter::new();
    image_filter.add_mime_type("image/*");
    image_filter.set_name(Some("Image File"));
    poster_picker.add_filter(&image_filter);

    let fetch_image_button = Button::new_with_label("Download");

    form.attach(&poster_label, 0, 2, 1, 1);
    form.attach(&poster_picker, 1, 2, 2, 1);
    form.attach(&fetch_image_button, 3, 2, 1, 1);

    let eps_label = Label::new(Some("Current Episode:"));
    eps_label.set_xalign(1.0);
    let curr_entry = gtk::Entry::new();
    curr_entry.set_hexpand(true);
    curr_entry.set_text(&format!("{}", show.current_ep));
    let slash_label = Label::new(Some("/"));
    let total_entry = gtk::Entry::new();
    total_entry.set_hexpand(true);
    total_entry.set_text(&format!("{}", show.total_eps));
    form.attach(&eps_label, 0, 3, 1, 1);
    form.attach(&curr_entry, 1, 3, 1, 1);
    form.attach(&slash_label, 2, 3, 1, 1);
    form.attach(&total_entry, 3, 3, 1, 1);

    let player_label = Label::new(Some("Video Player:"));
    player_label.set_xalign(1.0);
    let player_entry = gtk::Entry::new();
    player_entry.set_text(&show.player);
    form.attach(&player_label, 0, 4, 1, 1);
    form.attach(&player_entry, 1, 4, 3, 1);

    let regex_label = Label::new(Some("Regex:"));
    regex_label.set_xalign(1.0);
    let regex_entry = gtk::Entry::new();
    regex_entry.set_text(&show.regex);
    form.attach(&regex_label, 0, 5, 1, 1);
    form.attach(&regex_entry, 1, 5, 3, 1);

    let button_box = Box::new(Orientation::Horizontal, 0);
    let save_button = Button::new_with_label("Save");
    let cancel_button = Button::new_with_label("Cancel");

    button_box.pack_start(&save_button, true, true, 5);
    button_box.pack_start(&cancel_button, true, true, 5);

    main_box.pack_end(&button_box, false, false, 5);

    let w = window.clone();
    let s: Vec<Show> = orig_items.clone();
    let cset = settings.clone();
    cancel_button.connect_clicked(move |_| {
        if adding {
            main_screen(&w, &s, &cset);
        } else {
            view_screen(&w, &s, i, &cset);
        }
    });

    let sw = window.clone();
    let ss: Vec<Show> = items.clone();
    let sset = settings.clone();
    let spp = poster_picker.clone();
    let spap = path_picker.clone();
    let sne = name_entry.clone();
    save_button.connect_clicked(move |_| {
        let mut items = ss.clone();
        items[i].name = sne.get_text().unwrap();
        items[i].path = spap.get_filename().unwrap().as_path()
            .to_str().unwrap().to_string();
        items[i].poster_path = spp.get_filename().unwrap().as_path()
            .to_str().unwrap().to_string();
        items[i].current_ep = match (&curr_entry.get_text().unwrap()).parse::<i32>() {
            Ok(n) => n,
            Err(_) => 0,
        };
        items[i].total_eps = match (&total_entry.get_text().unwrap()).parse::<i32>() {
            Ok(n) => n,
            Err(_) => 0,
        };
        items[i].regex = regex_entry.get_text().unwrap();
        items[i].player = player_entry.get_text().unwrap();
        save_db(&sset, &items);

        view_screen(&sw, &items, i, &sset);
    });

    let fpp = poster_picker.clone();
    let fne = name_entry.clone();
    fetch_image_button.connect_clicked(move |_| {
        let ref name = fne.get_text().unwrap();
        let url = format!("{}search/multi?query={}&{}", TMDB, name, TMDB_KEY);
        println!("url: {}", url);
        let parsed = match json_get(&url) {
            Ok(text) => text,
            Err(e) => {
                // TODO: gtk dialog
                println!("{}", e);
                return;
            }
        };
        if parsed["results"].is_null() {
            println!("no results array");
            return;
        }
        let tmdb_base_url = match *TMDB_BASE_URL {
            Ok(ref a) => a,
            Err(ref e) => {
                // TODO: gtk dialog
                println!("can't get tmdb base url: {}", e);
                return;
            }
        };
        for r in parsed["results"].members() {
            let ref path = r["poster_path"];
            if path.is_null() {
                continue;
            }
            let path = match path.as_str() {
                Some(x) => x,
                None => continue
            };
            let image_url = format!("{}original{}", tmdb_base_url, path);
            println!("{}", image_url);
            let image_file = match https_get_bin(&image_url) {
                Ok(a) => a,
                Err(e) => {
                    // TODO: gtk dialog
                    println!("error downloading image: {}", e);
                    return;
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
                    println!("error opening image file for writing: {}", e);
                    return;
                }
            };
            match file.write_all(&image_file) {
                Ok(_) => (),
                Err(e) => {
                    println!("error saving image: {}", e);
                    return;
                }
            }
            fpp.set_filename(path);
            break;
        }
    });

    let fsne = name_entry.clone();
    let fspp = path_picker.clone();
    path_picker.connect_file_set(move |_| {
        let path = fspp.get_filename().unwrap().as_path()
            .to_str().unwrap().to_string();
        if fsne.get_text().unwrap() == "" {
            let dir_name = Regex::new(r".*/").unwrap().
                replace(&path, "").into_owned();
            let name = Regex::new(r"\[.*?\]").unwrap().
                replace_all(&dir_name, " ").into_owned();
            let name = Regex::new(r"_|-|\\.|[[:space:]]").unwrap().
                replace_all(&name, " ").into_owned();
            let name = Regex::new(r" +").unwrap().
                replace_all(&name, " ").into_owned();
            let name = name.trim();

            // stolen from the internetz
            fn title_case_word(s: &str) -> String {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().chain(
                        c.flat_map(|t| t.to_lowercase())).collect(),
                }
            }
            let name_words = name.split(' ').map(title_case_word).collect::<Vec<_>>();
            let name: String = name_words.join(" ");

            fsne.set_text(&name);
        }
    });

    window.show_all();
}

fn settings_screen(window: &Window, items: &Vec<Show>, settings: &Settings) {
    if let Some(child) = window.get_child() {
        child.destroy();
    };

    let main_box = Box::new(Orientation::Vertical, 20);
    main_box.set_margin_top(20);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_bottom(20);
    window.add(&main_box);

    let form = gtk::Grid::new();
    form.set_column_spacing(20);
    form.set_row_spacing(10);
    main_box.pack_start(&form, true, true, 5);

    let player_label = Label::new(Some("Video Player:"));
    player_label.set_xalign(1.0);
    let player_entry = gtk::Entry::new();
    player_entry.set_text(&settings.player);
    player_entry.set_hexpand(true);
    form.attach(&player_label, 0, 0, 1, 1);
    form.attach(&player_entry, 1, 0, 3, 1);

    let path_label = Label::new(Some("Default Path:"));
    path_label.set_xalign(1.0);
    let path_entry = gtk::Entry::new();
    path_entry.set_text(&settings.path);
    path_entry.set_hexpand(true);
    form.attach(&path_label, 0, 1, 1, 1);
    form.attach(&path_entry, 1, 1, 3, 1);

    let button_box = Box::new(Orientation::Horizontal, 0);
    let save_button = Button::new_with_label("Save");
    let cancel_button = Button::new_with_label("Cancel");

    button_box.pack_start(&save_button, true, true, 5);
    button_box.pack_start(&cancel_button, true, true, 5);

    main_box.pack_end(&button_box, false, false, 5);

    let cw = window.clone();
    let cs: Vec<Show> = items.clone();
    let cset = settings.clone();
    cancel_button.connect_clicked(move |_| {
        main_screen(&cw, &cs, &cset);
    });

    let sw = window.clone();
    let ss: Vec<Show> = items.clone();
    let sset = settings.clone();
    save_button.connect_clicked(move |_| {
        let mut set = sset.clone();
        set.player = player_entry.get_text().unwrap();
        set.path = path_entry.get_text().unwrap();
        save_db(&set, &ss);
        main_screen(&sw, &ss, &set);
    });

    window.show_all();
}

fn build_poster_list(window: &Window, button_box: &FlowBox,
                     items: &Vec<Show>, settings: &Settings,
                     filter: &str) {

    for child in button_box.get_children() {
        child.destroy();
    };

    let re = match Regex::new(&format!("(?i){}", filter)) {
        Ok(r) => r,
        Err(_) => Regex::new("").unwrap() // TODO: tell the user
    };
    for (index, item) in items.iter().enumerate() {
        if !re.is_match(&item.name) {
            continue;
        }
        let cover_event_box = EventBox::new();
        cover_event_box.set_events(
            gdk_sys::GDK_BUTTON_PRESS_MASK.bits() as i32);

        let cover_box = Box::new(Orientation::Vertical, 0);
        cover_event_box.add(&cover_box);
        cover_box.set_size_request(100, 200);

        // TODO: how do I unref?
        let image = match Pixbuf::new_from_file_at_size(
            &item.poster_path, 200, 200) {
            Ok(pixbuf) => Image::new_from_pixbuf(Some(&pixbuf)),
            Err(_) => Image::new_from_icon_name("gtk-missing-image", 1)
        };
        /*
        let pixbuf = Pixbuf::new_from_file_at_size(
            &item.poster_path, 200, 200)
            .unwrap();

        let image = Image::new_from_pixbuf(Some(&pixbuf));
        */
        //
        cover_box.pack_start(&image, false, true, 5);

        let l = Label::new(Some(&item.name));
        cover_box.pack_start(&l, false, true, 5);

        let w = window.clone();
        let s: Vec<Show> = items.clone();
        let i = index;
        let set = settings.clone();
        cover_event_box.connect_button_press_event(move |_, _| {
            view_screen(&w, &s, i, &set);
            Inhibit(false)
        });

        button_box.insert(&cover_event_box, -1);
    }
}

fn main_screen(window: &Window, items: &Vec<Show>, settings: &Settings) {
    if let Some(child) = window.get_child() {
        child.destroy();
    };

    let main_box = Box::new(Orientation::Vertical, 20);
    main_box.set_margin_top(20);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_bottom(20);

    // TITLE AND SETTINGS BUTTON
    let title_box = Box::new(Orientation::Horizontal, 0);
    let title_label = make_title_label(APP_TITLE);
    title_box.set_center_widget(Some(&title_label));

    let settings_button = Button::new_from_icon_name(
        "gtk-preferences", IconSize::Button.into());

    let sw = window.clone();
    let sitems: Vec<Show> = items.clone();
    let sset = settings.clone();
    settings_button.connect_clicked(move |_| {
        settings_screen(&sw, &sitems, &sset);
    });

    //let button = Button::new_with_label("Click me!");
    title_box.pack_end(&settings_button, false, true, 0);
    main_box.pack_start(&title_box, false, true, 5);
    //main_box.pack_start(&button, false, true, 5);

    // SEARCH BAR
    // TODO css
    let search_box = Box::new(Orientation::Horizontal, 0);
    main_box.pack_start(&search_box, false, true, 5);
    let search_bar = SearchEntry::new();
    let search_css_provider = gtk::CssProvider::new();
    let xdg_dirs = xdg::BaseDirectories::with_prefix("fucking-weeb").unwrap();
    let search_css_path = match xdg_dirs.find_data_file("search.css") {
        Some(p) => p.as_path().to_str().unwrap().to_owned(),
        None => "search.css".to_owned()
    };
    search_css_provider.load_from_path(&search_css_path).unwrap();
    let search_bar_style_context = search_bar.get_style_context().unwrap();
    search_bar_style_context.add_provider(&search_css_provider,
                                          gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);


    search_box.pack_start(&search_bar, true, true, 5);
    let add_button = Button::new_from_icon_name(
        "gtk-add", IconSize::Button.into());
    search_box.pack_start(&add_button, false, true, 0);

    let scrolled_window = ScrolledWindow::new(None, None);
    main_box.pack_start(&scrolled_window, true, true, 0);

    let viewport = Viewport::new(None, None);
    scrolled_window.add(&viewport);

    // POSTERS
    let button_box = FlowBox::new();
    button_box.set_selection_mode(SelectionMode::None);
    viewport.add(&button_box);

    build_poster_list(&window, &button_box, &items, &settings, "");

    let aw = window.clone();
    let ai: Vec<Show> = items.clone();
    let aset = settings.clone();
    add_button.connect_clicked(move |_| {
        edit_screen(&aw, &ai, None, &aset);
    });

    let sw = window.clone();
    let si: Vec<Show> = items.clone();
    let sbb = button_box.clone();
    let sset = settings.clone();
    search_bar.connect_search_changed(move |ref search_entry| {
        build_poster_list(&sw, &sbb, &si, &sset, &search_entry.get_text().unwrap());
        sw.show_all();
    });

    // MAIN
    window.add(&main_box);
    window.show_all();
}

fn load_db() -> WeebDB {
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

    let xdg_dirs = xdg::BaseDirectories::with_prefix("fucking-weeb").unwrap();
    let config_path = match xdg_dirs.find_config_file("fw-rs-db.json") {
        None => {
            println!("db file not found");
            return default_settings;
        },
        Some(path) => path
    };

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
    match rsjson::decode(&s) {
        Ok(a) => a,
        Err(e) => {
            println!("error decoding db json: {}", e.to_string());
            default_settings
        }
    }
}

fn save_db(settings: &Settings, items: &Vec<Show>) {
    let db = WeebDB {
        settings: settings.clone(),
        shows: items.clone(),
    };
    // TODO: rotate file for safety
    // what happens if the process is killed mid-write?
    let encoded = rsjson::as_pretty_json(&db);
    //println!("{}", encoded);
    // TODO: handle errors
    let xdg_dirs = xdg::BaseDirectories::with_prefix("fucking-weeb").unwrap();
    let config_path = xdg_dirs.place_config_file("fw-rs-db.json")
                          .expect("cannot create configuration directory");
    let mut file = File::create(config_path).expect("cannot create db file");
    match file.write_all(format!("{}\n", encoded).as_bytes()) {
        Ok(_) => (),
        Err(e) => {
            println!("Error saving db: {}", e);
        }
    };
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = Window::new(WindowType::Toplevel);
    window.set_title(APP_TITLE);
    window.set_default_size(570, 600);

    let db = load_db();
    let shows = db.shows;
    let settings = db.settings;

    let w = window.clone();

    main_screen(&w, &shows, &settings);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}


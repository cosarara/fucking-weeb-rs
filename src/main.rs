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

use rustc_serialize::json;

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

static APP_TITLE : &'static str = "Fucking Weeb";



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

fn watch(show: &Show) {
    let ref dir = show.path;
    let ref reg = show.regex;
    let ep = show.current_ep.abs() as u32;
    match find_ep(&dir, ep, &reg) {
        Ok(path) => {
            println!("{}", path.to_str().unwrap());
            //let cmd =
            Command::new("mpv")
                .arg(path.to_str().unwrap())
                .spawn()
                .expect("couldn't launch mpv");
        },
        Err(e) => println!("{}", e) // TODO: gtk dialog
    }
}

fn view_screen(window: &Window, items: &Vec<Show>, i: usize) {
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
    let title_label = Label::new(Some(&show.name));
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
    back_button.connect_clicked(move |_| {
        let mut items = bs.clone();
        let ep = bspin.get_value_as_int();
        items[i].current_ep = ep;
        main_screen(&bw, &items);
    });

    let dw = window.clone();
    let mut ds = items.clone();
    ds.remove(i);
    // FIXME: prompt confirmation
    remove_button.connect_clicked(move |_| {
        save_db(&ds);
        main_screen(&dw, &ds);
    });

    let ew = window.clone();
    let es = items.clone();
    let espin = spin.clone();
    edit_button.connect_clicked(move |_| {
        let mut items = es.clone();
        let ep = espin.get_value_as_int();
        items[i].current_ep = ep;
        edit_screen(&ew, &items, Some(i));
    });

    let wspin = spin.clone();
    let wshow = show.clone();

    watch_button.connect_clicked(move |_| {
        let ep = wspin.get_value_as_int();
        let mut show = wshow.clone();
        show.current_ep = ep;
        watch(&show);
    });

    let wnspin = spin.clone();
    let wnshow = show.clone();
    watch_next_button.connect_clicked(move |_| {
        let mut show = wnshow.clone();
        let ep = wnspin.get_value_as_int();
        let max_ep = show.total_eps;
        let ep = if ep < max_ep { ep+1 } else { ep };
        wnspin.set_value(ep as f64);
        show.current_ep = ep;
        watch(&show);
    });

    let vcspin = spin.clone();
    let vcitems = items.clone();
    spin.connect_value_changed(move |_| {
        let ep = vcspin.get_value_as_int();
        //println!("{}", ep);
        let mut items = vcitems.clone();
        items[i].current_ep = ep;
        save_db(&items);
    });

    window.show_all();
}

fn edit_screen(window: &Window, items: &Vec<Show>, i: Option<usize>) {
    let mut items = items.clone();
    let show = match i {
        Some(i) => items[i].clone(),
        None => {
            let s = Show {
                name: "".to_string(),
                // TODO: defaults
                path: "".to_string(),
                poster_path: "".to_string(),
                current_ep: 1,
                total_eps: 24,
                regex: "".to_string(),
                player: "".to_string(),
            };
            items.push(s.clone());
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
    let s : Vec<Show> = items.clone();
    cancel_button.connect_clicked(move |_| {
        view_screen(&w, &s, i);
    });

    let sw = window.clone();
    let ss : Vec<Show> = items.clone();
    save_button.connect_clicked(move |_| {
        let mut items = ss.clone();
        items[i].name = name_entry.get_text().unwrap();
        items[i].path = path_picker.get_filename().unwrap().as_path()
            .to_str().unwrap().to_string();
        items[i].poster_path = poster_picker.get_filename().unwrap().as_path()
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

        view_screen(&sw, &items, i);
    });

    window.show_all();
}

fn main_screen(window: &Window, items: &Vec<Show>) {
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
    // TODO fonts
    let title_label = Label::new(Some(APP_TITLE));
    title_box.set_center_widget(Some(&title_label));

    let settings_button = Button::new_from_icon_name(
        "gtk-preferences", IconSize::Button.into());
    settings_button.connect_clicked(|_| {
        println!("going to settings screen TODO");
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
    search_box.pack_start(&search_bar, true, true, 5);
    // TODO connect search bar
    let add_button = Button::new_from_icon_name(
        "gtk-add", IconSize::Button.into());
    search_box.pack_start(&add_button, false, true, 0);
    // TODO connect add button

    let scrolled_window = ScrolledWindow::new(None, None);
    main_box.pack_start(&scrolled_window, true, true, 0);

    let viewport = Viewport::new(None, None);
    scrolled_window.add(&viewport);

    // POSTERS
    let button_box = FlowBox::new();
    button_box.set_selection_mode(SelectionMode::None);
    viewport.add(&button_box);

    for (index, item) in items.iter().enumerate() {
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
        let s : Vec<Show> = items.clone();
        let i = index;
        cover_event_box.connect_button_press_event(move |_, _| {
            view_screen(&w, &s, i);
            Inhibit(false)
        });

        button_box.insert(&cover_event_box, -1);
    }

    let aw = window.clone();
    let ai : Vec<Show> = items.clone();
    add_button.connect_clicked(move |_| {
        edit_screen(&aw, &ai, None);
    });

    // MAIN
    window.add(&main_box);
    window.show_all();
}

fn load_db() -> Vec<Show> {
    let default_items = vec![
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
    ];

    let mut file = match File::open("fw-rs-db.json") {
        Err(e) => {
            println!("error opening db file: {}", e.to_string());
            return default_items
        },
        Ok(file) => file
    };
    let mut s = String::new();
    match file.read_to_string(&mut s) {
        Err(e) => {
            println!("error reading db: {}", e.to_string());
            return default_items;
        }
        Ok(_) => ()
    }
    match json::decode(&s) {
        Ok(a) => a,
        Err(e) => {
            println!("error decoding db json: {}", e.to_string());
            default_items
        }
    }
}

fn save_db(items: &Vec<Show>) {
    // TODO: rotate file for safety
    // what happens if the process is killed mid-write?
    let encoded = json::as_pretty_json(&items);
    //println!("{}", encoded);
    let mut file = File::create("fw-rs-db.json").unwrap();
    file.write_all(format!("{}\n", encoded).as_bytes()).unwrap();
}

fn main() {
    if gtk::init().is_err() {
        println!("Failed to initialize GTK.");
        return;
    }

    let window = Window::new(WindowType::Toplevel);
    window.set_title(APP_TITLE);
    window.set_default_size(570, 600);

    let items = load_db();


    let w = window.clone();
    //
    main_screen(&w, &items);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}


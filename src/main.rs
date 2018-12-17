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
extern crate gtk_sys;
extern crate gdk_sys;
extern crate gdk_pixbuf;
extern crate glib;
extern crate soup_sys;
extern crate regex;
extern crate xdg;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;
#[macro_use] extern crate log;

// TMDB
// yes we have 2 different jsons k
extern crate json;

#[macro_use]
extern crate lazy_static;

use gtk::prelude::*;

use gtk::{Button, Window, WindowType, Box, Orientation,
    Label, IconSize, SearchEntry, ScrolledWindow, Viewport,
    FlowBox, SelectionMode, EventBox, Image, MessageDialog,
    MessageType, DialogFlags, ButtonsType};

use gdk_pixbuf::Pixbuf;

use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use regex::Regex;

mod db;
use db::*;

mod tmdb;
use tmdb::*;

mod soup;

use std::cell::RefCell;
use std::sync::mpsc::{channel, Receiver};

static APP_TITLE: &'static str = "Fucking Weeb";

// declare a new thread local storage key
// TODO: proper struct
thread_local!(
    static GLOBAL: RefCell<Option<(gtk::FileChooserButton,
                                   gtk::Button,
                                   gtk::Window,
                                   Receiver<Result<String,String>>)>> = RefCell::new(None)
);


fn gtk_err(window: &Window, message: &str) {
    println!("{}", message);
    let dialog = MessageDialog::new(Some(window), DialogFlags::MODAL,
        MessageType::Error, ButtonsType::Ok, message);
    dialog.run();
    dialog.destroy();
}

fn find_ep(dir: &str, num: u32, regex: &str) -> Result<PathBuf, String> {
    let dir = std::path::Path::new(dir);
    if !dir.is_dir() {
        return Err("not a directory".to_string());
    }
    let files = fs::read_dir(dir)
        .map_err(|e| e.to_string())?;
    // gotta be separate cuz rust dumb
    let files: Result<Vec<_>, _> = files.collect();
    let files = files.map_err(|e| e.to_string())?;
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
        let re = Regex::new(&r).map_err(|e| e.to_string())?;
        let mut best_match: Option<&Path> = None;
        let mut best_score = 0; // lower is better
        for file in files.iter().map(|x| x.as_path()) {
            //let file_name = file.to_str().unwrap();
            let file_name = file.file_name().and_then(|s| s.to_str()).unwrap();
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

fn watch(show: &Show, player: &str) -> Result<(), String> {
    let ref dir = show.path;
    let ref reg = show.regex;
    let ep = show.current_ep.abs() as u32;
    let path = find_ep(&dir, ep, &reg)?;
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
    cmd.map_err(|e| e.to_string())?;
    Ok(())
}

fn make_title_label(text: &str) -> Label {
    let label = Label::new(None);
    label.set_markup(&format!("<span weight='bold' size='xx-large'>{}</span>", text));
    label
}

fn drop_cover(items: &Vec<Show>, i: usize, settings: &Settings,
              text: &str) -> Vec<Show> {
    let re = Regex::new("(.*)://(.*)").unwrap();
    if re.find(&text).is_none() {
        return items.clone();
    }
    let caps = re.captures(&text).unwrap();
    let protocol = &caps[1];
    let path = &caps[2];
    println!("protocol: {}", protocol);
    println!("body: {}", path);
    let mut shows = items.clone();
    if protocol == "file" {
        shows[i].poster_path = path.to_string();
    } else if protocol == "http" || protocol == "https" {
        let lpath = match download_image(&text) {
            Ok(p) => p,
            Err(e) => {
                println!("{}", e);
                return items.clone();
            }
        };
        shows[i].poster_path = lpath;
    } else {
        return items.clone();
    }
    save_db(&settings, &shows);
    return shows;
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
    let targets = vec![gtk::TargetEntry::new("STRING", gtk::TargetFlags::empty(), 0),
                       gtk::TargetEntry::new("text/plain", gtk::TargetFlags::empty(), 0)];
    cover_event_box.drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::COPY);

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
    let total_label = Label::new(Some(total_label_text.as_str()));
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

    let wbw = window.clone();
    let wspin = spin.clone();
    let wshow = show.clone();
    let wplayer = settings.player.clone();
    watch_button.connect_clicked(move |_| {
        let ep = wspin.get_value_as_int();
        let mut show = wshow.clone();
        show.current_ep = ep;
        watch(&show, &wplayer).unwrap_or_else(|e| gtk_err(&wbw, &e));
    });

    let wnbw = window.clone();
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
        watch(&show, &wnplayer).unwrap_or_else(|e| gtk_err(&wnbw, &e));
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

    let cew = window.clone();
    let ceitems = items.clone();
    let ceset = settings.clone();
    cover_event_box.connect_drag_data_received(move |_, _, _, _, data, _, _| {
        let text = match data.get_text() {
            Some(text) => text,
            None => { return; }
        };
        let shows = drop_cover(&ceitems, i, &ceset, &text);
        view_screen(&cew, &shows, i, &ceset);
    });

    window.show_all();
}

fn fetch_image(name: &str) -> Result<String, String> {
    let url = format!("{}search/multi?query={}&{}", TMDB, name, TMDB_KEY);
    println!("url: {}", url);
    let parsed = json_get(&url).map_err(|e| e.to_string())?;
    if parsed["results"].is_null() {
        return Err("no results array".to_string());
    }
    let tmdb_base_url = match *TMDB_BASE_URL {
        Ok(ref a) => a,
        Err(ref e) => {
            return Err(format!("can't get tmdb base url: {}", e));
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
        let path = download_image(&image_url).map_err(|e| e.to_string())?;
        return Ok(path);
    }
    return Err("Image not found".to_string());
}

fn receive() -> glib::Continue {
    GLOBAL.with(|global| {
        if let Some((ref picker, ref button,
                     ref window, ref rx)) = *global.borrow() {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(text) => {
                        picker.set_filename(&text);
                    },
                    Err(e) => { gtk_err(&window, &e); }
                }
                button.set_sensitive(true);
            }
        }
    });
    glib::Continue(false)
}

fn download_thread(window: &Window, button: &gtk::Button, picker: &gtk::FileChooserButton, name: &str) {
    let name = name.to_string();
    let (tx, rx) = channel();
    GLOBAL.with(move |global| {
        *global.borrow_mut() = Some((picker.clone(), button.clone(), window.clone(), rx))
    });

    std::thread::spawn(move || {
        match fetch_image(&name) {
            Err(e) => {
                tx.send(Err(e))
            }, //gtk_err(&fw, &e),
            Ok(path) => {
                tx.send(Ok(path))
            }
        }.expect("Couldn't send data to channel");
        glib::idle_add(receive);
    });
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
    gtk::FileFilterExt::set_name(&image_filter, Some("Image File"));
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
        GLOBAL.with(move |global| {
            *global.borrow_mut() = None
        });
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
        GLOBAL.with(move |global| {
            *global.borrow_mut() = None
        });

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
    let fw = window.clone();
    let fib = fetch_image_button.clone();

    fetch_image_button.connect_clicked(move |_| {
        fib.set_sensitive(false);
        let name = fne.get_text().unwrap().to_string();
        download_thread(&fw, &fib, &fpp, &name);
        println!("download thread started");
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


    let mut items = items.clone();
    items.sort_by_key(|x| {x.name.clone()});

    for (index, item) in items.iter().enumerate() {
        if !re.is_match(&item.name) {
            continue;
        }
        let cover_event_box = EventBox::new();
        cover_event_box.set_events(gdk_sys::GDK_BUTTON_PRESS_MASK as i32);

        let cover_box = Box::new(Orientation::Vertical, 0);
        cover_event_box.add(&cover_box);
        cover_box.set_size_request(100, 200);

        let targets = vec![gtk::TargetEntry::new("STRING", gtk::TargetFlags::empty(), 0),
                           gtk::TargetEntry::new("text/plain", gtk::TargetFlags::empty(), 0)];
        cover_event_box.drag_dest_set(gtk::DestDefaults::ALL, &targets, gdk::DragAction::COPY);

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

        let l = Label::new(Some(item.name.as_str()));
        l.set_line_wrap(true);
        l.set_max_width_chars(18);
        cover_box.pack_start(&l, false, true, 5);

        let w = window.clone();
        let s: Vec<Show> = items.clone();
        let set = settings.clone();
        cover_event_box.connect_button_press_event(move |_, _| {
            view_screen(&w, &s, index, &set);
            Inhibit(false)
        });

        let cew = window.clone();
        let ceitems = items.clone();
        let ceset = settings.clone();
        cover_event_box.connect_drag_data_received(move |_, _, _, _, data, _, _| {
            let text = match data.get_text() {
                Some(text) => text,
                None => { return; }
            };
            let shows = drop_cover(&ceitems, index, &ceset, &text);
            main_screen(&cew, &shows, &ceset);
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
    match search_css_provider.load_from_path(&search_css_path) {
        Err(e) => println!("error loading css: {}", e),
        Ok(_) => ()
    }
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


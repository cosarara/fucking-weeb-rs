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
            let cmd = Command::new("mpv")
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

    let pixbuf = Pixbuf::new_from_file_at_size(
        &show.poster_path, 300, 300)
        .unwrap();

    // TODO: use cairo here, to have it resize automagically
    let image = Image::new_from_pixbuf(Some(&pixbuf));
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

        // FIXME
        let pixbuf = Pixbuf::new_from_file_at_size(
            &item.poster_path, 200, 200)
            .unwrap();

        let image = Image::new_from_pixbuf(Some(&pixbuf));
        // TODO: how do I unref?
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
        },
        Show {
            name: "Neon Genesis Evangelion".to_string(),
            path: "/home/jaume/videos/series/0-Sorted/neon_genesis_evangelion-1080p-renewal_cat/".to_string(),
            poster_path: "/home/jaume/.local/share/fucking-weeb/Neon%20Genesis%20Evangelion.jpg".to_string(),
            current_ep: 5,
            total_eps: 26,
            regex: "".to_string(),
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


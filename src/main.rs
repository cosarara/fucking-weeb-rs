extern crate gtk;
extern crate gdk_sys;
extern crate gdk_pixbuf;

use gtk::prelude::*;

use gtk::{Button, Window, WindowType, Box, Orientation,
    Label, IconSize, SearchEntry, ScrolledWindow, Viewport,
    FlowBox, SelectionMode, EventBox, Image};

use gdk_pixbuf::Pixbuf;

struct Show {
    name: String,
    path: String,
    poster_path: String,
    current_ep: i32,
    total_eps: i32,
}

static app_title : &'static str = "Fucking Weeb";

fn main_screen(window : &Window) {
    let main_box = Box::new(Orientation::Vertical, 20);
    main_box.set_margin_top(20);
    main_box.set_margin_start(20);
    main_box.set_margin_end(20);
    main_box.set_margin_bottom(20);

    // TITLE AND SETTINGS BUTTON
    let title_box = Box::new(Orientation::Horizontal, 0);
    // TODO fonts
    let title_label = Label::new(Some(app_title));
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

    let items = vec![
        Show {
            name: "Ranma".to_string(),
            path: "/home/jaume/videos/series/1-More/Ranma/".to_string(),
            poster_path: "/home/jaume/.local/share/fucking-weeb/ranma.jpg".to_string(),
            current_ep: 25,
            total_eps: 150
        },
        Show {
            name: "Neon Genesis Evangelion".to_string(),
            path: "/home/jaume/videos/series/0-Sorted/neon_genesis_evangelion-1080p-renewal_cat/".to_string(),
            poster_path: "/home/jaume/.local/share/fucking-weeb/Neon%20Genesis%20Evangelion.jpg".to_string(),
            current_ep: 5,
            total_eps: 26
        }
    ];

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

        let c_name = item.name.clone();
        cover_event_box.connect_button_press_event(move |_, _| {
            println!("pressed some poster {} {}", index, c_name);
            Inhibit(false)
        });

        button_box.insert(&cover_event_box, -1);
    }

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
    window.set_title(app_title);
    window.set_default_size(350, 70);

    main_screen(&window);

    window.connect_delete_event(|_, _| {
        gtk::main_quit();
        Inhibit(false)
    });

    gtk::main();
}


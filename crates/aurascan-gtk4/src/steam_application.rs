use glib::prelude::*;
use glib::subclass::prelude::*;
use gtk::glib;

mod imp {
    use std::cell::{Cell, RefCell};

    use super::*;

    #[derive(Debug, Default, glib::Properties)]
    #[properties(wrapper_type = super::SteamApplication)]
    pub struct SteamApplication {
        #[property(get, set)]
        pub id: Cell<u64>,
        #[property(get, set)]
        pub name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for SteamApplication {
        const NAME: &'static str = "AscSteamApplication";
        type Type = super::SteamApplication;
    }

    #[glib::derived_properties]
    impl ObjectImpl for SteamApplication {}
}

glib::wrapper! {
    pub struct SteamApplication(ObjectSubclass<imp::SteamApplication>);
}

impl SteamApplication {
    pub fn new(id: u64, name: &str) -> Self {
        glib::Object::builder()
            .property("id", id)
            .property("name", name)
            .build()
    }
}

use glib::subclass::prelude::*;
use gtk::glib;

mod imp {
    use std::cell::RefCell;

    use super::*;

    #[derive(Debug, Default)]
    pub struct ProtonApplication {
        pub id: RefCell<String>,
        pub name: RefCell<String>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for ProtonApplication {
        const NAME: &'static str = "AscProtonApplication";
        type Type = super::ProtonApplication;
    }

    impl ObjectImpl for ProtonApplication {}
}

glib::wrapper! {
    pub struct ProtonApplication(ObjectSubclass<imp::ProtonApplication>);
}

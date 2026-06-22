mod application;
mod config;
mod widgets;

use crate::application::Application;
use crate::config::{app_id, base_id, resources_file};
use gtk::gio::prelude::ApplicationExtManual;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    let res = gio::Resource::load(resources_file()).expect("Could not load gresource file");
    gio::resources_register(&res);

    let app = Application::new();
    app.run()
}

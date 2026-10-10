mod application;
mod config;
mod widgets;

use crate::application::Application;
use crate::config::app_id;
use gtk::gio::prelude::ApplicationExtManual;
use gtk::{gio, glib};

fn main() -> glib::ExitCode {
    gio::resources_register_include!("tech.jeandudey.Aurascan.gresource")
        .expect("Failed to register resources");
    let app = Application::new();
    app.run()
}

mod app;
mod application;
mod pipeline2;

use crate::app::AppModel;
use crate::application::Application;
use relm4::RelmApp;

const CSS: &str = include_str!("app.css");

fn main() {
    let application = Application::new();

    let app = RelmApp::from_app(application);
    //relm4::set_global_css(CSS);
    app.run::<AppModel>(());
    unsafe { gst::deinit() };
}

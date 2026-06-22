mod app;
mod application;
mod pipeline2;

use crate::app::AppModel;
use crate::application::Application;
use relm4::RelmApp;

fn main() {
    let application = Application::new();

    let app = RelmApp::from_app(application);
    app.run::<AppModel>(());
    unsafe { gst::deinit() };
}

use crate::app::AppModel;
use relm4::RelmApp;

mod app;
mod pipeline2;

const CSS: &str = include_str!("app.css");

fn main() {
    gst::init().unwrap();
    aurascan_gtk4::init();
    adw::init().unwrap();

    gstgtk4::plugin_register_static().unwrap();
    gstaurascan::plugin_register_static().unwrap();

    let app = RelmApp::new("tech.jeandudey.AuraScan");
    relm4::set_global_css(CSS);
    app.run::<AppModel>(());
    unsafe { gst::deinit() };
}

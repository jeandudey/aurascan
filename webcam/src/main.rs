use crate::app::AppModel;
use relm4::RelmApp;

//use crate::face_tracker::FaceTracker;
//use gst::prelude::*;
//use gst_analytics::AnalyticsMetaRefExt;
//use gst_analytics::prelude::*;
//use gstburnextra::BackendType;
//use gstburnextra::scrfd::ModelType;
//use std::sync::Mutex;

mod app;
mod pipeline;

fn main() {
    gst::init().unwrap();
    gtk::init().unwrap();
    adw::init().unwrap();

    gstgtk4::plugin_register_static().unwrap();
    gstburnextra::plugin_register_static().unwrap();

    let app = RelmApp::new("tech.jeandudey.AuraScan");
    app.run::<AppModel>(());
    unsafe { gst::deinit() };
}

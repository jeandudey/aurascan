use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct VideoCropScale(ObjectSubclass<imp::VideoCropScale>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "videocropscale",
        gst::Rank::NONE,
        VideoCropScale::static_type(),
    )
}

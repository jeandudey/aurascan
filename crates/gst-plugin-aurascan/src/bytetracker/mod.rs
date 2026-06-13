use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct ByteTracker(ObjectSubclass<imp::ByteTracker>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "bytetracker",
        gst::Rank::NONE,
        ByteTracker::static_type(),
    )
}

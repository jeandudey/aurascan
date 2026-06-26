use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct FaceSelector(ObjectSubclass<imp::FaceSelector>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "faceselector",
        gst::Rank::NONE,
        FaceSelector::static_type(),
    )
}

use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct ScrfdTensorDec(ObjectSubclass<imp::ScrfdTensorDec>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "scrfdtensordec",
        gst::Rank::NONE,
        ScrfdTensorDec::static_type(),
    )
}

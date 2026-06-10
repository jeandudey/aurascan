use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct SixDRepNet360TensorDec(ObjectSubclass<imp::SixDRepNet360TensorDec>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "sixdrepnet360tensordec",
        gst::Rank::NONE,
        SixDRepNet360TensorDec::static_type(),
    )
}

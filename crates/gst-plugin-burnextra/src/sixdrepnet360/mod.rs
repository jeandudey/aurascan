use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct SixDRepNet360Inference(ObjectSubclass<imp::SixDRepNet360Inference>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "burnextra-sixdrepnet360inference",
        gst::Rank::NONE,
        SixDRepNet360Inference::static_type(),
    )
}

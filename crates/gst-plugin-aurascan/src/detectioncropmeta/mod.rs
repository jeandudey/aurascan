use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct DetectionCropMeta(ObjectSubclass<imp::DetectionCropMeta>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "detectioncropmeta",
        gst::Rank::NONE,
        DetectionCropMeta::static_type(),
    )
}

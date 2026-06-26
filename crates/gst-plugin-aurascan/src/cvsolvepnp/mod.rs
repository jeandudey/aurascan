use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct CvSolvePnp(ObjectSubclass<imp::SolvePnp>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "cvsolvepnp",
        gst::Rank::NONE,
        CvSolvePnp::static_type(),
    )
}

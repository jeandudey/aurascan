use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct FacePnpAdapter(ObjectSubclass<imp::FacePnpAdapter>)
        @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "facepnpadapter",
        gst::Rank::NONE,
        FacePnpAdapter::static_type(),
    )
}

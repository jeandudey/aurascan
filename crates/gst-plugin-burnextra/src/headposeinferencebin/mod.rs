use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct HeadPoseInferenceBin(ObjectSubclass<imp::HeadPoseInferenceBin>)
        @extends gst::Bin, gst::Element, gst::Object,
        @implements gst::ChildProxy;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "headposeinferencebin",
        gst::Rank::NONE,
        HeadPoseInferenceBin::static_type(),
    )
}

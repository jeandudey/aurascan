use gst::glib;
use gst::prelude::*;

pub mod imp;

#[derive(Copy, Clone, Default, PartialEq, Eq, glib::Enum)]
#[repr(C)]
#[enum_type(name = "GstAurascanScrfdModelType")]
pub enum ModelType {
    #[default]
    Scrfd500m,
    Scrfd500mKps,
    Scrfd1g,
    Scrfd2_5g,
    Scrfd2_5gKps,
    Scrfd10g,
    Scrfd10gKps,
    Scrfd34g,
}

impl From<ModelType> for scrfd_burn::ModelType {
    fn from(value: ModelType) -> Self {
        match value {
            ModelType::Scrfd500m => scrfd_burn::ModelType::Scrfd500m,
            ModelType::Scrfd500mKps => scrfd_burn::ModelType::Scrfd500mKps,
            ModelType::Scrfd1g => scrfd_burn::ModelType::Scrfd1g,
            ModelType::Scrfd2_5g => scrfd_burn::ModelType::Scrfd2_5g,
            ModelType::Scrfd2_5gKps => scrfd_burn::ModelType::Scrfd2_5gKps,
            ModelType::Scrfd10g => scrfd_burn::ModelType::Scrfd10g,
            ModelType::Scrfd10gKps => scrfd_burn::ModelType::Scrfd10gKps,
            ModelType::Scrfd34g => scrfd_burn::ModelType::Scrfd34g,
        }
    }
}

glib::wrapper! {
    pub struct ScrfdInference(ObjectSubclass<imp::ScrfdInference>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    ModelType::static_type().mark_as_plugin_api(gst::PluginAPIFlags::empty());

    gst::Element::register(
        Some(plugin),
        "burn-scrfdinference",
        gst::Rank::NONE,
        ScrfdInference::static_type(),
    )
}

use gst::glib;
use gst::prelude::*;

pub mod imp;

pub const SCRFD_GROUP_ID: &glib::GStr = glib::gstr!("scrfd");
pub const SCRFD_KPS_GROUP_ID: &glib::GStr = glib::gstr!("scrfd-kps");
pub const SCRFD_SCORE8_OUT_ID: &glib::GStr = glib::gstr!("scrfd-score8-out");
pub const SCRFD_SCORE16_OUT_ID: &glib::GStr = glib::gstr!("scrfd-score16-out");
pub const SCRFD_SCORE32_OUT_ID: &glib::GStr = glib::gstr!("scrfd-score32-out");
pub const SCRFD_BBOX8_OUT_ID: &glib::GStr = glib::gstr!("scrfd-bbox8-out");
pub const SCRFD_BBOX16_OUT_ID: &glib::GStr = glib::gstr!("scrfd-bbox16-out");
pub const SCRFD_BBOX32_OUT_ID: &glib::GStr = glib::gstr!("scrfd-bbox32-out");
pub const SCRFD_KPS8_OUT_ID: &glib::GStr = glib::gstr!("scrfd-kps8-out");
pub const SCRFD_KPS16_OUT_ID: &glib::GStr = glib::gstr!("scrfd-kps16-out");
pub const SCRFD_KPS32_OUT_ID: &glib::GStr = glib::gstr!("scrfd-kps32-out");

glib::wrapper! {
    pub struct ScrfdInference(ObjectSubclass<imp::ScrfdInference>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

#[derive(Copy, Clone, Default, PartialEq, Eq, glib::Enum)]
#[repr(C)]
#[enum_type(name = "GstAscScrfdModelType")]
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

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, glib::Enum)]
#[repr(C)]
#[enum_type(name = "GstAurascanScrfdModelKind")]
pub enum ModelKind {
    #[default]
    Normal,
    Kps,
}

impl ModelKind {
    /// Parse the SCRFD model kind from the caps.
    pub fn from_caps(caps: &gst::Caps) -> Option<Self> {
        let structure = caps.structure(0)?;
        let tensors = structure.get::<gst::Structure>("tensors").ok()?;

        if tensors.has_field(SCRFD_KPS_GROUP_ID) {
            Some(ModelKind::Kps)
        } else if tensors.has_field(SCRFD_GROUP_ID) {
            Some(ModelKind::Normal)
        } else {
            None
        }
    }
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    ModelType::static_type().mark_as_plugin_api(gst::PluginAPIFlags::empty());
    ModelKind::static_type().mark_as_plugin_api(gst::PluginAPIFlags::empty());

    gst::Element::register(
        Some(plugin),
        "burn-scrfdinference",
        gst::Rank::NONE,
        ScrfdInference::static_type(),
    )
}

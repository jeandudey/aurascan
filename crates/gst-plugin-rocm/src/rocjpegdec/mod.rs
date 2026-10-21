// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MPL-2.0

use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    pub struct RocJpegDec(ObjectSubclass<imp::RocJpegDec>)
        @extends gst_video::VideoDecoder, gst::Element, gst::Object;
}

/// Backend used by the decoder.
#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, glib::Enum)]
#[repr(C)]
#[enum_type(name = "GstRocJpegDecBackend")]
pub enum Backend {
    #[default]
    #[enum_value(name = "Hardware: VCN JPEG engine", nick = "hardware")]
    Hardware = 0,
    #[enum_value(name = "Hybrid: CPU and GPU", nick = "hybrid")]
    Hybrid = 1,
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "rocjpegdec",
        gst::Rank::NONE,
        RocJpegDec::static_type(),
    )
}

// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::sync::{LazyLock, Once};

use gst::prelude::*;

mod camera;
mod device_provider;
mod head_pose_view;
mod pipeline_tee;
mod steam_application;
mod viewfinder;
mod wgpu_area;

pub use camera::Camera;
pub use device_provider::{DeviceProvider, DeviceProviderError};
pub use head_pose_view::HeadPoseView;
pub use pipeline_tee::PipelineTee;
pub use steam_application::SteamApplication;
pub use viewfinder::{Viewfinder, ViewfinderState};
pub use wgpu_area::WGPUArea;

pub(crate) const PREFERRED_FORMATS: [&str; 5] = ["I420", "BGRx", "RGBx", "xBGR", "xRGB"];
pub(crate) const SUPPORTED_ENCODINGS: [&str; 2] = ["video/x-raw", "image/jpeg"];
pub(crate) const MAXIMUM_RATE: i32 = 30;

/// Supported caps for the app, already frame capped.
pub(crate) static SUPPORTED_CAPS: LazyLock<gst::Caps> = LazyLock::new(|| {
    crate::SUPPORTED_ENCODINGS
        .iter()
        .map(|enc| {
            let framerate_range = gst::FractionRange::new(
                gst::Fraction::new(0, 1),
                gst::Fraction::new(crate::MAXIMUM_RATE, 1),
            );
            gst::Caps::builder(*enc)
                .field("framerate", framerate_range)
                .build()
        })
        .collect()
});

pub static IR_CAPS: LazyLock<gst::Caps> = LazyLock::new(|| {
    crate::SUPPORTED_ENCODINGS
        .iter()
        .map(|enc| {
            gst::Caps::builder(*enc)
                .field("format", gst_video::VideoFormat::Gray8.to_str())
                .build()
        })
        .collect()
});

static IS_INIT: Once = Once::new();

pub fn init() {
    IS_INIT.call_once(|| {
        gtk::init().unwrap();
        adw::init().unwrap();
        gst::init().unwrap();

        gstaurascan::plugin_register_static().unwrap();
        gstgtk4::plugin_register_static().unwrap();

        Camera::static_type();
        DeviceProvider::static_type();
        HeadPoseView::static_type();
        PipelineTee::static_type();
        SteamApplication::static_type();
        Viewfinder::static_type();
        WGPUArea::static_type();
    });
}

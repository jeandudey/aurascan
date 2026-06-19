// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use gst::prelude::*;
use std::sync::Once;

mod device_provider;
mod head_pose_view;
mod wgpu_area;

pub use head_pose_view::HeadPoseView;
pub use wgpu_area::WGPUArea;

static IS_INIT: Once = Once::new();

pub fn init() {
    IS_INIT.call_once(|| {
        gtk::init().unwrap();

        HeadPoseView::static_type();
        WGPUArea::static_type();
    });
}

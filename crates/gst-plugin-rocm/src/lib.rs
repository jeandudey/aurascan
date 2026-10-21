// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MPL-2.0

use gst::glib;

pub mod rocjpegdec;

fn plugin_init(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    rocjpegdec::register(plugin)
}

gst::plugin_define!(
    rocm,
    env!("CARGO_PKG_DESCRIPTION"),
    plugin_init,
    concat!(env!("CARGO_PKG_VERSION"), "-", env!("COMMIT_ID")),
    "MPL-2.0",
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_NAME"),
    env!("CARGO_PKG_REPOSITORY"),
    env!("BUILD_REL_DATE")
);

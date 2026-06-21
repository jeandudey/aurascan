// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashMap;

use gst::prelude::DeviceExt;
use gtk::glib;
use gtk::prelude::*;
use gtk::subclass::prelude::*;

mod imp {
    use std::cell::OnceCell;

    use glib::Properties;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Camera)]
    pub struct Camera {
        #[property(get, set, construct_only)]
        device: OnceCell<gst::Device>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "AscCamera";
        type Type = super::Camera;
    }

    #[glib::derived_properties]
    impl ObjectImpl for Camera {}
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>);
}

impl Camera {
    pub(crate) fn new(device: &gst::Device) -> Self {
        glib::Object::builder().property("device", device).build()
    }

    pub(crate) fn reconfigure(&self, element: &gst::Element) -> Result<(), glib::BoolError> {
        self.device().reconfigure_element(element)
    }

    pub(crate) fn create_element(
        &self,
        client_name: &str,
    ) -> Result<gst::Element, glib::BoolError> {
        let element = self.device().create_element(None)?;
        element.set_property("client-name", client_name);
        Ok(element)
    }

    /// Gets the `serial` of the device
    ///
    /// For newer pipewire versions this corresponds to the `target-object` of
    /// the element and for older versions this corresponds to the `path` of the
    /// device.
    pub(crate) fn target_object(&self) -> Option<u64> {
        let device = self.device();
        if device.has_property_with_type("serial", u64::static_type()) {
            Some(device.property::<u64>("serial"))
        } else {
            None
        }
    }

    pub(crate) fn best_caps(&self) -> gst::Caps {
        let caps = self
            .caps()
            .unwrap_or_else(|| gst::Caps::builder("video/x-raw").build());
        let highest_res_caps = filter_caps(caps);
        log::debug!("Using caps: {highest_res_caps:#?}");

        highest_res_caps
    }

    /// Gets the display name of the camera represented by `self`.
    ///
    /// # Returns
    ///
    /// the display name.
    pub fn display_name(&self) -> glib::GString {
        self.device().display_name()
    }

    /// Gets the user-set nickname of the camera represented by `self`.
    ///
    /// # Returns
    ///
    /// the display name if set.
    pub fn nick(&self) -> Option<String> {
        self.device().properties().and_then(|properties| {
            properties
                .value("node.nick")
                .ok()
                .and_then(|value| value.get::<String>().ok())
        })
    }

    /// Gets the supported [`caps`](https://gstreamer.freedesktop.org/documentation/additional/design/caps.html)
    /// of the camera represented by `self`.
    ///
    /// # Returns
    ///
    /// The available caps if available.
    pub fn caps(&self) -> Option<gst::Caps> {
        self.device().caps().as_ref().map(limit_fps)
    }

    /// Gets all the available properties for the camera represented by `self`.
    ///
    /// # Returns
    ///
    /// a [`HashMap`][std::collections::HashMap], with the property name as the
    /// key and a [`GValue`][gtk::glib::Value] as the value.
    pub fn properties(&self) -> HashMap<String, glib::SendValue> {
        self.device()
            .properties()
            .map(|s| {
                s.iter()
                    .map(|(key, val)| (key.to_string(), val.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }
}

fn limit_fps(caps: &gst::Caps) -> gst::Caps {
    caps.intersect_with_mode(&crate::SUPPORTED_CAPS, gst::CapsIntersectMode::First)
}

// For each resolution and format we only keep the highest resolution.
fn filter_caps(caps: gst::Caps) -> gst::Caps {
    let mut best_caps = gst::Caps::new_empty();
    caps.iter().for_each(|s| {
        if let Some(framerate) = framerate_from_structure(s) {
            let best = best_resolution_for_fps(&caps, framerate);
            let best_sorted = sort_caps_for_preferred_formats(best);
            best_caps.merge(best_sorted);
        }
    });

    best_caps.merge(caps);
    best_caps
}

fn framerate_from_structure(structure: &gst::StructureRef) -> Option<gst::Fraction> {
    // TODO Handle gst::List and gst::Array
    if let Ok(framerate) = structure.get::<gst::Fraction>("framerate") {
        Some(framerate)
    } else if let Ok(range) = structure.get::<gst::FractionRange>("framerate") {
        Some(range.max())
    } else if let Ok(array) = structure.get::<gst::Array>("framerate") {
        array
            .iter()
            .filter_map(|s| s.get::<gst::Fraction>().ok())
            .filter(|frac| frac <= &gst::Fraction::new(crate::MAXIMUM_RATE, 1))
            .max()
    } else if let Ok(array) = structure.get::<gst::List>("framerate") {
        array
            .iter()
            .filter_map(|s| s.get::<gst::Fraction>().ok())
            .filter(|frac| frac <= &gst::Fraction::new(crate::MAXIMUM_RATE, 1))
            .max()
    } else {
        None
    }
}

fn best_resolution_for_fps(caps: &gst::Caps, framerate: gst::Fraction) -> gst::Caps {
    let fixed_caps = crate::SUPPORTED_ENCODINGS
        .iter()
        .map(|encoding| {
            gst::Caps::builder(*encoding)
                .field("framerate", framerate)
                .build()
        })
        .collect::<gst::Caps>();
    let caps_with_format = caps.intersect_with_mode(&fixed_caps, gst::CapsIntersectMode::First);

    // We try to find the biggest height smaller than `MAX_HEIGHT`p.
    if let Some(Size { height, width }) = best_mode(&caps_with_format) {
        let fixed_res = crate::SUPPORTED_ENCODINGS
            .iter()
            .map(|encoding| {
                gst::Caps::builder(*encoding)
                    .field("width", width)
                    .field("height", height)
                    .build()
            })
            .collect::<gst::Caps>();

        caps_with_format.intersect_with_mode(&fixed_res, gst::CapsIntersectMode::First)
    } else {
        caps_with_format
    }
}

fn sort_caps_for_preferred_formats(caps: gst::Caps) -> gst::Caps {
    let preferred_format_caps = crate::PREFERRED_FORMATS
        .iter()
        .map(|format| {
            gst::Caps::builder("video/x-raw")
                .field("format", *format)
                .build()
        })
        .collect::<gst::Caps>();

    let mut sorted_caps =
        preferred_format_caps.intersect_with_mode(&caps, gst::CapsIntersectMode::First);
    sorted_caps.merge(caps);
    sorted_caps
}

#[derive(Debug, PartialEq)]
struct Size {
    pub width: i32,
    pub height: i32,
}

fn best_mode(caps: &gst::Caps) -> Option<Size> {
    const MIN_WIDTH: i32 = 640;
    const MIN_HEIGHT: i32 = 480;
    const MAX_HEIGHT: i32 = 1080;
    const OPTIMAL_RATIO: f32 = 16.0 / 9.0;

    let mut best_size_optimal_ratio: Option<Size> = None;
    let mut best_size_any_ratio: Option<Size> = None;
    let mut best_size_fallback: Option<Size> = None;

    for cap in caps.iter() {
        let Ok(width) = cap.get::<i32>("width") else {
            continue;
        };
        let Ok(height) = cap.get::<i32>("height") else {
            continue;
        };

        if best_size_fallback.is_none() {
            best_size_fallback = Some(Size { width, height });
        }

        let max_width = (height as f32 * OPTIMAL_RATIO).ceil() as i32;
        if (MIN_WIDTH..=max_width).contains(&width) && (MIN_HEIGHT..=MAX_HEIGHT).contains(&height) {
            if width == (height as f32 * OPTIMAL_RATIO) as i32 {
                if let Some(Size {
                    width: best_w,
                    height: best_h,
                }) = best_size_optimal_ratio
                {
                    if width >= best_w && height >= best_h {
                        best_size_optimal_ratio = Some(Size { width, height });
                    }
                } else {
                    best_size_optimal_ratio = Some(Size { width, height });
                }
            } else if let Some(Size {
                width: best_w,
                height: best_h,
            }) = best_size_any_ratio
            {
                if width >= best_w && height >= best_h {
                    best_size_any_ratio = Some(Size { width, height });
                }
            } else {
                best_size_any_ratio = Some(Size { width, height });
            }
        }
    }

    best_size_optimal_ratio
        .or(best_size_any_ratio)
        .or(best_size_fallback)
}

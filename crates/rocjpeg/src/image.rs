// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use crate::MAX_COMPONENTS;

/// Destination planes of a decode operation, in device memory.
#[derive(Debug, Clone, Copy)]
pub struct Image {
    channel: [*mut u8; MAX_COMPONENTS],
    pitch: [u32; MAX_COMPONENTS],
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Image {
    pub fn new() -> Self {
        Self {
            channel: [std::ptr::null_mut(); MAX_COMPONENTS],
            pitch: [0; MAX_COMPONENTS],
        }
    }

    /// Sets the device pointer and pitch (bytes per row) of plane `index`.
    pub fn set_plane(&mut self, index: usize, device_ptr: *mut u8, pitch: u32) {
        self.channel[index] = device_ptr;
        self.pitch[index] = pitch;
    }
}

impl From<Image> for rocjpeg_sys::RocJpegImage {
    fn from(image: Image) -> Self {
        Self {
            channel: image.channel,
            pitch: image.pitch,
        }
    }
}

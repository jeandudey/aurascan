// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use crate::OutputFormat;

/// Crop rectangle (region of interest) in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

/// Parameters of a decode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeParams {
    pub output_format: OutputFormat,
    pub crop: Option<Rect>,
}

impl DecodeParams {
    pub fn new(output_format: OutputFormat) -> Self {
        Self {
            output_format,
            crop: None,
        }
    }
}

impl From<DecodeParams> for rocjpeg_sys::RocJpegDecodeParams {
    fn from(params: DecodeParams) -> Self {
        let mut ffi_params = Self {
            output_format: params.output_format.into(),
            ..Self::default()
        };
        if let Some(crop) = params.crop {
            ffi_params.crop_rectangle.left = crop.left;
            ffi_params.crop_rectangle.top = crop.top;
            ffi_params.crop_rectangle.right = crop.right;
            ffi_params.crop_rectangle.bottom = crop.bottom;
        }
        ffi_params
    }
}

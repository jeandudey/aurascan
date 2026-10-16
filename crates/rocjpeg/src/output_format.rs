// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

/// Output format of the decoded image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    /// The unchanged image of the decoder: planar YUV for 4:4:4 and 4:4:0, packed YUYV for
    /// 4:2:2, NV12 (Y + interleaved UV) for 4:2:0 and Y for 4:0:0.
    Native,
    /// Planar Y, U and V (only Y for 4:0:0).
    YuvPlanar,
    /// Luma only.
    Y,
    /// Interleaved RGB.
    Rgb,
    /// Planar RGB.
    RgbPlanar,
}

impl From<OutputFormat> for rocjpeg_sys::RocJpegOutputFormat {
    fn from(format: OutputFormat) -> Self {
        match format {
            OutputFormat::Native => Self::ROCJPEG_OUTPUT_NATIVE,
            OutputFormat::YuvPlanar => Self::ROCJPEG_OUTPUT_YUV_PLANAR,
            OutputFormat::Y => Self::ROCJPEG_OUTPUT_Y,
            OutputFormat::Rgb => Self::ROCJPEG_OUTPUT_RGB,
            OutputFormat::RgbPlanar => Self::ROCJPEG_OUTPUT_RGB_PLANAR,
        }
    }
}

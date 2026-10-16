// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use crate::{ChromaSubsampling, MAX_COMPONENTS, OutputFormat, PlaneLayout};

/// Information about a parsed JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageInfo {
    pub num_components: u8,
    pub subsampling: ChromaSubsampling,
    pub widths: [u32; MAX_COMPONENTS],
    pub heights: [u32; MAX_COMPONENTS],
}

impl ImageInfo {
    pub fn width(&self) -> u32 {
        self.widths[0]
    }

    pub fn height(&self) -> u32 {
        self.heights[0]
    }

    /// The planes the decoder writes for `format`, or `None` if the combination isn't known
    /// (4:1:1 and unknown subsampling in the YUV formats).
    ///
    /// For a crop, use the size of the crop rectangle instead of the image size.
    pub fn plane_layout(&self, format: OutputFormat) -> Option<Vec<PlaneLayout>> {
        use ChromaSubsampling as C;
        let (w, h) = (self.width() as usize, self.height() as usize);
        let plane = |row_bytes, rows| PlaneLayout { row_bytes, rows };
        Some(match format {
            OutputFormat::Rgb => vec![plane(w * 3, h)],
            OutputFormat::RgbPlanar => vec![plane(w, h); 3],
            OutputFormat::Y => vec![plane(w, h)],
            OutputFormat::YuvPlanar => match self.subsampling {
                C::Yuv400 => vec![plane(w, h)],
                C::Yuv444 | C::Yuv440 | C::Yuv422 | C::Yuv420 => {
                    let (cw, ch) = (self.widths[1] as usize, self.heights[1] as usize);
                    vec![plane(w, h), plane(cw, ch), plane(cw, ch)]
                }
                _ => return None,
            },
            OutputFormat::Native => match self.subsampling {
                C::Yuv400 => vec![plane(w, h)],
                C::Yuv444 => vec![plane(w, h); 3],
                C::Yuv440 => vec![plane(w, h), plane(w, h / 2), plane(w, h / 2)],
                C::Yuv422 => vec![plane(w * 2, h)],
                C::Yuv420 => vec![plane(w, h), plane(w, h / 2)],
                _ => return None,
            },
        })
    }
}

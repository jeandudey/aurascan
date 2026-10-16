// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

/// Chroma subsampling of a JPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromaSubsampling {
    Yuv444,
    Yuv440,
    Yuv422,
    Yuv420,
    Yuv411,
    Yuv400,
    Unknown,
}

impl ChromaSubsampling {
    pub(crate) fn from_ffi(css: rocjpeg_sys::RocJpegChromaSubsampling) -> Self {
        use rocjpeg_sys::RocJpegChromaSubsampling as C;
        match css {
            C::ROCJPEG_CSS_444 => Self::Yuv444,
            C::ROCJPEG_CSS_440 => Self::Yuv440,
            C::ROCJPEG_CSS_422 => Self::Yuv422,
            C::ROCJPEG_CSS_420 => Self::Yuv420,
            C::ROCJPEG_CSS_411 => Self::Yuv411,
            C::ROCJPEG_CSS_400 => Self::Yuv400,
            C::ROCJPEG_CSS_UNKNOWN => Self::Unknown,
        }
    }
}

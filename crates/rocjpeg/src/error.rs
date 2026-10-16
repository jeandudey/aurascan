// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::ffi::CStr;
use std::fmt;

use rocjpeg_sys::RocJpegStatus;

/// An error returned by rocJPEG.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Error(pub RocJpegStatus);

impl Error {
    /// The raw rocJPEG status code.
    pub fn status(&self) -> RocJpegStatus {
        self.0
    }

    /// Whether the JPEG itself is invalid or unsupported by the decoder.
    pub fn is_bad_jpeg(&self) -> bool {
        matches!(
            self.0,
            RocJpegStatus::ROCJPEG_STATUS_BAD_JPEG
                | RocJpegStatus::ROCJPEG_STATUS_JPEG_NOT_SUPPORTED
        )
    }

    /// Whether the hardware decoder isn't available (try [`Backend::Hybrid`](crate::Backend::Hybrid)).
    pub fn is_hw_decoder_unsupported(&self) -> bool {
        matches!(
            self.0,
            RocJpegStatus::ROCJPEG_STATUS_HW_JPEG_DECODER_NOT_SUPPORTED
        )
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = unsafe { CStr::from_ptr(rocjpeg_sys::rocJpegGetErrorName(self.0)) };
        write!(f, "{}", name.to_string_lossy())
    }
}

impl std::error::Error for Error {}

/// Conversion of a rocJPEG status code into a [`Result`].
pub trait IntoResult {
    /// `Ok(())` for `ROCJPEG_STATUS_SUCCESS`, the status as an [`Error`] otherwise.
    fn into_result(self) -> Result<(), Error>;
}

impl IntoResult for RocJpegStatus {
    fn into_result(self) -> Result<(), Error> {
        if self == RocJpegStatus::ROCJPEG_STATUS_SUCCESS {
            Ok(())
        } else {
            Err(Error(self))
        }
    }
}

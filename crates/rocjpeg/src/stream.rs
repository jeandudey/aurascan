// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use rocjpeg_sys::RocJpegStatus;

use crate::Error;
use crate::IntoResult;

/// A parsed JPEG stream.
///
/// rocJPEG keeps pointers into the JPEG data instead of copying it, so the stream borrows it
/// for as long as it is used.
pub struct Stream<'a> {
    pub(crate) handle: rocjpeg_sys::RocJpegStreamHandle,
    _data: PhantomData<&'a [u8]>,
}

// SAFETY: the handle is owned and only used through `&self`/`&mut self` methods.
unsafe impl Send for Stream<'_> {}

impl<'a> Stream<'a> {
    /// Parses the headers of a JPEG image.
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        let length = u32::try_from(data.len())
            .map_err(|_| Error(RocJpegStatus::ROCJPEG_STATUS_INVALID_PARAMETER))?;
        let mut handle = std::ptr::null_mut();
        let status = unsafe { rocjpeg_sys::rocJpegStreamCreate(&mut handle) };
        status.into_result()?;
        // From here on `Drop` releases the handle.
        let stream = Self {
            handle,
            _data: PhantomData,
        };
        let status = unsafe {
            rocjpeg_sys::rocJpegStreamParse(data.as_ptr(), length as usize, stream.handle)
        };
        status.into_result()?;
        Ok(stream)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        unsafe { rocjpeg_sys::rocJpegStreamDestroy(self.handle) };
    }
}

// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::marker::PhantomData;

use rocjpeg_sys::RocJpegStatus;

use crate::{Error, result};

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
        if u32::try_from(data.len()).is_err() {
            return Err(Error(RocJpegStatus::ROCJPEG_STATUS_INVALID_PARAMETER));
        }
        // SAFETY: the handle is destroyed by `Drop` and `data` is borrowed for `'a`.
        let handle = unsafe { result::stream_create() }?;
        // From here on `Drop` releases the handle.
        let stream = Self {
            handle,
            _data: PhantomData,
        };
        unsafe { result::stream_parse(stream.handle, data) }?;
        Ok(stream)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        // SAFETY: the handle is live and not used afterwards.
        let _ = unsafe { result::stream_destroy(self.handle) };
    }
}

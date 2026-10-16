// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use crate::{Backend, ChromaSubsampling, DecodeParams, Error, Image, ImageInfo, Stream, result};

/// A rocJPEG decoder bound to a GPU.
///
/// The HIP current device is thread local and rocJPEG only selects it when the decoder is
/// created, so decode on the thread that created the decoder or make the device current
/// (`hipSetDevice`) first.
pub struct Decoder {
    handle: rocjpeg_sys::RocJpegHandle,
}

// SAFETY: rocJPEG serializes calls on a handle with an internal mutex.
unsafe impl Send for Decoder {}

impl Decoder {
    pub fn new(backend: Backend, device_id: i32) -> Result<Self, Error> {
        // SAFETY: the handle is destroyed by `Drop`.
        let handle = unsafe { result::create(backend.into(), device_id) }?;
        Ok(Self { handle })
    }

    /// Dimensions and format of a parsed JPEG.
    pub fn image_info(&self, stream: &Stream<'_>) -> Result<ImageInfo, Error> {
        // SAFETY: both handles are live and the stream is parsed.
        let info = unsafe { result::get_image_info(self.handle, stream.handle) }?;
        Ok(ImageInfo {
            num_components: info.num_components,
            subsampling: ChromaSubsampling::from_ffi(info.subsampling),
            widths: info.widths,
            heights: info.heights,
        })
    }

    /// Decodes `stream` into `destination`. Returns when the decoded data is complete.
    ///
    /// # Safety
    ///
    /// Every plane required by `params.output_format` (see [`ImageInfo::plane_layout`]) must
    /// be set in `destination`, point to device memory that is valid for writes of
    /// `pitch * rows` bytes, and not be used by anything else during the call.
    pub unsafe fn decode(
        &self,
        stream: &Stream<'_>,
        params: &DecodeParams,
        destination: &mut Image,
    ) -> Result<(), Error> {
        let params = (*params).into();
        let mut image = (*destination).into();
        unsafe { result::decode(self.handle, stream.handle, &params, &mut image) }
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        // SAFETY: the handle is live and not used afterwards.
        let _ = unsafe { result::destroy(self.handle) };
    }
}

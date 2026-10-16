// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use crate::{
    Backend, ChromaSubsampling, DecodeParams, Error, Image, ImageInfo, IntoResult, MAX_COMPONENTS,
    Stream,
};

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
        let mut handle = std::ptr::null_mut();
        let status = unsafe { rocjpeg_sys::rocJpegCreate(backend.into(), device_id, &mut handle) };
        status.into_result()?;
        Ok(Self { handle })
    }

    /// Dimensions and format of a parsed JPEG.
    pub fn image_info(&self, stream: &Stream<'_>) -> Result<ImageInfo, Error> {
        let mut num_components = 0u8;
        let mut subsampling = rocjpeg_sys::RocJpegChromaSubsampling::ROCJPEG_CSS_UNKNOWN;
        let mut widths = [0u32; MAX_COMPONENTS];
        let mut heights = [0u32; MAX_COMPONENTS];
        let status = unsafe {
            rocjpeg_sys::rocJpegGetImageInfo(
                self.handle,
                stream.handle,
                &mut num_components,
                &mut subsampling,
                widths.as_mut_ptr(),
                heights.as_mut_ptr(),
            )
        };
        status.into_result()?;
        Ok(ImageInfo {
            num_components,
            subsampling: ChromaSubsampling::from_ffi(subsampling),
            widths,
            heights,
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
        let status =
            unsafe { rocjpeg_sys::rocJpegDecode(self.handle, stream.handle, &params, &mut image) };
        status.into_result()
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { rocjpeg_sys::rocJpegDestroy(self.handle) };
    }
}

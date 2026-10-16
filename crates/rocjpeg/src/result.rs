// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! Thin, unsafe wrappers over the rocJPEG C functions that turn the status code into a [`Result`] and
//! return the values the function initializes instead of taking out-parameters.
//!
//! The wrappers only check the status: all other safety requirements of the C functions are
//! left to the caller.

use std::mem::MaybeUninit;

use rocjpeg_sys::{
    RocJpegBackend, RocJpegChromaSubsampling, RocJpegDecodeParams, RocJpegHandle, RocJpegImage,
    RocJpegStreamHandle,
};

use crate::{Error, IntoResult, MAX_COMPONENTS};

/// `rocJpegStreamCreate`.
///
/// # Safety
///
/// rocJPEG must be usable (see the library documentation).
pub unsafe fn stream_create() -> Result<RocJpegStreamHandle, Error> {
    let mut handle = MaybeUninit::uninit();
    unsafe { rocjpeg_sys::rocJpegStreamCreate(handle.as_mut_ptr()) }.into_result()?;
    // SAFETY: initialized by `rocJpegStreamCreate`, which succeeded.
    Ok(unsafe { handle.assume_init() })
}

/// `rocJpegStreamParse`.
///
/// # Safety
///
/// `handle` must be a live stream handle. rocJPEG keeps pointers into `data`, so it must
/// outlive every use of the stream.
pub unsafe fn stream_parse(handle: RocJpegStreamHandle, data: &[u8]) -> Result<(), Error> {
    unsafe { rocjpeg_sys::rocJpegStreamParse(data.as_ptr(), data.len(), handle) }.into_result()
}

/// `rocJpegStreamDestroy`.
///
/// # Safety
///
/// `handle` must be a live stream handle that isn't used afterwards.
pub unsafe fn stream_destroy(handle: RocJpegStreamHandle) -> Result<(), Error> {
    unsafe { rocjpeg_sys::rocJpegStreamDestroy(handle) }.into_result()
}

/// `rocJpegCreate`.
///
/// # Safety
///
/// rocJPEG must be usable (see the library documentation).
pub unsafe fn create(backend: RocJpegBackend, device_id: i32) -> Result<RocJpegHandle, Error> {
    let mut handle = MaybeUninit::uninit();
    unsafe { rocjpeg_sys::rocJpegCreate(backend, device_id, handle.as_mut_ptr()) }.into_result()?;
    // SAFETY: initialized by `rocJpegCreate`, which succeeded.
    Ok(unsafe { handle.assume_init() })
}

/// `rocJpegDestroy`.
///
/// # Safety
///
/// `handle` must be a live decoder handle that isn't used afterwards.
pub unsafe fn destroy(handle: RocJpegHandle) -> Result<(), Error> {
    unsafe { rocjpeg_sys::rocJpegDestroy(handle) }.into_result()
}

/// The values returned by [`get_image_info`].
pub struct RawImageInfo {
    pub num_components: u8,
    pub subsampling: RocJpegChromaSubsampling,
    pub widths: [u32; MAX_COMPONENTS],
    pub heights: [u32; MAX_COMPONENTS],
}

/// `rocJpegGetImageInfo`.
///
/// # Safety
///
/// `handle` and `stream` must be live handles, and `stream` must have been parsed.
pub unsafe fn get_image_info(
    handle: RocJpegHandle,
    stream: RocJpegStreamHandle,
) -> Result<RawImageInfo, Error> {
    let mut num_components = MaybeUninit::uninit();
    let mut subsampling = MaybeUninit::uninit();
    // rocJPEG only writes the entries of the components the image has, so these start zeroed.
    let mut widths = [0u32; MAX_COMPONENTS];
    let mut heights = [0u32; MAX_COMPONENTS];
    unsafe {
        rocjpeg_sys::rocJpegGetImageInfo(
            handle,
            stream,
            num_components.as_mut_ptr(),
            subsampling.as_mut_ptr(),
            widths.as_mut_ptr(),
            heights.as_mut_ptr(),
        )
    }
    .into_result()?;
    // SAFETY: initialized by `rocJpegGetImageInfo`, which succeeded.
    Ok(unsafe {
        RawImageInfo {
            num_components: num_components.assume_init(),
            subsampling: subsampling.assume_init(),
            widths,
            heights,
        }
    })
}

/// `rocJpegDecode`.
///
/// # Safety
///
/// `handle` and `stream` must be live handles, `stream` must have been parsed, and every plane
/// of `destination` required by `params` must point to device memory that is valid for writes
/// of `pitch * rows` bytes.
pub unsafe fn decode(
    handle: RocJpegHandle,
    stream: RocJpegStreamHandle,
    params: &RocJpegDecodeParams,
    destination: &mut RocJpegImage,
) -> Result<(), Error> {
    unsafe { rocjpeg_sys::rocJpegDecode(handle, stream, params, destination) }.into_result()
}

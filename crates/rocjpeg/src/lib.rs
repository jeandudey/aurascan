// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! Safe bindings to
//! [rocJPEG](https://github.com/ROCm/rocm-systems/tree/develop/projects/rocjpeg), the AMD VCN
//! hardware JPEG decoder.
//!
//! Decoded images are written to **device (HIP) memory** provided by the caller through
//! [`Image`]. [`rocJpegDecode`](Decoder::decode) is synchronous: the output is complete when
//! it returns.
//!
//! Typical usage:
//!
//! ```no_run
//! use rocjpeg::{Backend, DecodeParams, Decoder, Image, OutputFormat, Stream};
//!
//! # let jpeg: &[u8] = &[];
//! let decoder = Decoder::new(Backend::Hardware, 0)?;
//! let stream = Stream::parse(jpeg)?;
//! let info = decoder.image_info(&stream)?;
//! let planes = info.plane_layout(OutputFormat::Rgb).unwrap();
//! // Allocate `planes[0].size()` bytes of device memory, then:
//! # let device_ptr = std::ptr::null_mut();
//! let mut image = Image::new();
//! image.set_plane(0, device_ptr, planes[0].row_bytes as u32);
//! unsafe { decoder.decode(&stream, &DecodeParams::new(OutputFormat::Rgb), &mut image)? };
//! # Ok::<(), rocjpeg::Error>(())
//! ```

mod backend;
mod chroma_subsampling;
mod decode_params;
mod decoder;
mod error;
mod image;
mod image_info;
mod output_format;
mod plane_layout;
pub mod result;
mod stream;

pub use backend::Backend;
pub use chroma_subsampling::ChromaSubsampling;
pub use decode_params::{DecodeParams, Rect};
pub use decoder::Decoder;
pub use error::{Error, IntoResult};
pub use image::Image;
pub use image_info::ImageInfo;
pub use output_format::OutputFormat;
pub use plane_layout::PlaneLayout;
pub use rocjpeg_sys;
pub use stream::Stream;

/// Maximum number of image planes.
pub const MAX_COMPONENTS: usize = rocjpeg_sys::ROCJPEG_MAX_COMPONENT as usize;

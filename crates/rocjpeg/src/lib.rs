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

use std::marker::PhantomData;

mod error;

pub use error::Error;
use error::check;
pub use rocjpeg_sys;
use rocjpeg_sys::RocJpegStatus;

/// Maximum number of image planes.
pub const MAX_COMPONENTS: usize = rocjpeg_sys::ROCJPEG_MAX_COMPONENT as usize;

/// Decoder backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Use the VCN hardware JPEG engine.
    #[default]
    Hardware,
    /// Hybrid CPU/GPU decoding.
    Hybrid,
}

impl From<Backend> for rocjpeg_sys::RocJpegBackend {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Hardware => Self::ROCJPEG_BACKEND_HARDWARE,
            Backend::Hybrid => Self::ROCJPEG_BACKEND_HYBRID,
        }
    }
}

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
    fn from_ffi(css: rocjpeg_sys::RocJpegChromaSubsampling) -> Self {
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

/// Crop rectangle (region of interest) in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i16,
    pub top: i16,
    pub right: i16,
    pub bottom: i16,
}

/// Parameters of a decode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeParams {
    pub output_format: OutputFormat,
    pub crop: Option<Rect>,
}

impl DecodeParams {
    pub fn new(output_format: OutputFormat) -> Self {
        Self {
            output_format,
            crop: None,
        }
    }
}

impl From<DecodeParams> for rocjpeg_sys::RocJpegDecodeParams {
    fn from(params: DecodeParams) -> Self {
        // SAFETY: all fields are integers or enums with a valid zero value.
        let mut ffi_params: Self = unsafe { std::mem::zeroed() };
        ffi_params.output_format = params.output_format.into();
        if let Some(crop) = params.crop {
            ffi_params.crop_rectangle.left = crop.left;
            ffi_params.crop_rectangle.top = crop.top;
            ffi_params.crop_rectangle.right = crop.right;
            ffi_params.crop_rectangle.bottom = crop.bottom;
        }
        ffi_params
    }
}

/// Size of one plane of a decoded image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneLayout {
    /// Bytes per row without padding. The pitch passed to [`Image::set_plane`] must be at
    /// least this.
    pub row_bytes: usize,
    /// Number of rows.
    pub rows: usize,
}

impl PlaneLayout {
    /// Size of the plane in bytes when using `row_bytes` as pitch.
    pub fn size(&self) -> usize {
        self.row_bytes * self.rows
    }
}

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

/// Destination planes of a decode operation, in device memory.
#[derive(Debug, Clone, Copy)]
pub struct Image {
    channel: [*mut u8; MAX_COMPONENTS],
    pitch: [u32; MAX_COMPONENTS],
}

impl Default for Image {
    fn default() -> Self {
        Self::new()
    }
}

impl Image {
    pub fn new() -> Self {
        Self {
            channel: [std::ptr::null_mut(); MAX_COMPONENTS],
            pitch: [0; MAX_COMPONENTS],
        }
    }

    /// Sets the device pointer and pitch (bytes per row) of plane `index`.
    pub fn set_plane(&mut self, index: usize, device_ptr: *mut u8, pitch: u32) {
        self.channel[index] = device_ptr;
        self.pitch[index] = pitch;
    }
}

impl From<Image> for rocjpeg_sys::RocJpegImage {
    fn from(image: Image) -> Self {
        Self {
            channel: image.channel,
            pitch: image.pitch,
        }
    }
}

/// A parsed JPEG stream.
///
/// rocJPEG keeps pointers into the JPEG data instead of copying it, so the stream borrows it
/// for as long as it is used.
pub struct Stream<'a> {
    handle: rocjpeg_sys::RocJpegStreamHandle,
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
        check(unsafe { rocjpeg_sys::rocJpegStreamCreate(&mut handle) })?;
        // From here on `Drop` releases the handle.
        let stream = Self {
            handle,
            _data: PhantomData,
        };
        check(unsafe {
            rocjpeg_sys::rocJpegStreamParse(data.as_ptr(), length as usize, stream.handle)
        })?;
        Ok(stream)
    }
}

impl Drop for Stream<'_> {
    fn drop(&mut self) {
        unsafe { rocjpeg_sys::rocJpegStreamDestroy(self.handle) };
    }
}

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
        check(unsafe { rocjpeg_sys::rocJpegCreate(backend.into(), device_id, &mut handle) })?;
        Ok(Self { handle })
    }

    /// Dimensions and format of a parsed JPEG.
    pub fn image_info(&self, stream: &Stream<'_>) -> Result<ImageInfo, Error> {
        let mut num_components = 0u8;
        let mut subsampling = rocjpeg_sys::RocJpegChromaSubsampling::ROCJPEG_CSS_UNKNOWN;
        let mut widths = [0u32; MAX_COMPONENTS];
        let mut heights = [0u32; MAX_COMPONENTS];
        check(unsafe {
            rocjpeg_sys::rocJpegGetImageInfo(
                self.handle,
                stream.handle,
                &mut num_components,
                &mut subsampling,
                widths.as_mut_ptr(),
                heights.as_mut_ptr(),
            )
        })?;
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
        check(unsafe {
            rocjpeg_sys::rocJpegDecode(self.handle, stream.handle, &params, &mut image)
        })
    }
}

impl Drop for Decoder {
    fn drop(&mut self) {
        unsafe { rocjpeg_sys::rocJpegDestroy(self.handle) };
    }
}

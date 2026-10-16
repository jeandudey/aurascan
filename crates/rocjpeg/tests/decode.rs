// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! The decode tests need an AMD GPU with a VCN JPEG engine and the sample images shipped by
//! the rocjpeg package (`$CONDA_PREFIX/share/rocjpeg/images`). Run them with
//! `cargo test -p rocjpeg -- --ignored`.

use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};
use std::path::PathBuf;

use rocjpeg::{
    Backend, ChromaSubsampling, DecodeParams, Decoder, Image, OutputFormat, Rect, Stream,
};

#[link(name = "amdhip64")]
unsafe extern "C" {
    fn hipMalloc(ptr: *mut *mut c_void, size: usize) -> c_int;
    fn hipFree(ptr: *mut c_void) -> c_int;
    fn hipMemset(ptr: *mut c_void, value: c_int, size: usize) -> c_int;
    fn hipMemcpy(dst: *mut c_void, src: *const c_void, size: usize, kind: c_uint) -> c_int;
}

const GUARD: usize = 4096;
const POISON: u8 = 0xA5;

/// Device buffer followed by a poisoned guard area to detect writes past the end.
struct DeviceBuffer {
    ptr: *mut c_void,
    size: usize,
}

impl DeviceBuffer {
    fn new(size: usize) -> Self {
        let mut ptr = std::ptr::null_mut();
        assert_eq!(unsafe { hipMalloc(&mut ptr, size + GUARD) }, 0);
        assert_eq!(unsafe { hipMemset(ptr, POISON as c_int, size + GUARD) }, 0);
        Self { ptr, size }
    }

    fn read(&self) -> Vec<u8> {
        let mut out = vec![0u8; self.size + GUARD];
        assert_eq!(
            unsafe { hipMemcpy(out.as_mut_ptr().cast(), self.ptr, out.len(), 2) },
            0
        );
        out
    }
}

impl Drop for DeviceBuffer {
    fn drop(&mut self) {
        unsafe { hipFree(self.ptr) };
    }
}

fn sample(name: &str) -> Option<Vec<u8>> {
    let prefix = std::env::var_os("CONDA_PREFIX")?;
    let path = PathBuf::from(prefix)
        .join("share/rocjpeg/images")
        .join(name);
    std::fs::read(path).ok()
}

const SAMPLES: &[(&str, ChromaSubsampling)] = &[
    ("mug_420.jpg", ChromaSubsampling::Yuv420),
    ("mug_422.jpg", ChromaSubsampling::Yuv422),
    ("mug_400.jpg", ChromaSubsampling::Yuv400),
];

#[test]
fn bad_jpeg_is_rejected() {
    let err = Stream::parse(&[0u8; 16]).err().expect("parse must fail");
    assert!(err.is_bad_jpeg(), "{err}");
}

#[test]
#[ignore = "requires an AMD GPU"]
fn image_info() {
    let decoder = Decoder::new(Backend::Hardware, 0).unwrap();
    for (name, subsampling) in SAMPLES {
        let Some(data) = sample(name) else {
            panic!("{name} not found")
        };
        let stream = Stream::parse(&data).unwrap();
        let info = decoder.image_info(&stream).unwrap();
        assert_eq!((info.width(), info.height()), (3840, 2160), "{name}");
        assert_eq!(info.subsampling, *subsampling, "{name}");
    }
}

#[test]
#[ignore = "requires an AMD GPU"]
fn decode_fills_exactly_the_planes_of_the_layout() {
    let decoder = Decoder::new(Backend::Hardware, 0).unwrap();
    let formats = [
        OutputFormat::Native,
        OutputFormat::YuvPlanar,
        OutputFormat::Y,
        OutputFormat::Rgb,
        OutputFormat::RgbPlanar,
    ];
    for (name, _) in SAMPLES {
        let Some(data) = sample(name) else {
            panic!("{name} not found")
        };
        let stream = Stream::parse(&data).unwrap();
        let info = decoder.image_info(&stream).unwrap();
        for format in formats {
            let layout = info.plane_layout(format).unwrap();
            let buffers: Vec<_> = layout.iter().map(|p| DeviceBuffer::new(p.size())).collect();
            let mut image = Image::new();
            for (i, (plane, buf)) in layout.iter().zip(&buffers).enumerate() {
                image.set_plane(i, buf.ptr.cast(), plane.row_bytes as u32);
            }
            unsafe { decoder.decode(&stream, &DecodeParams::new(format), &mut image) }
                .unwrap_or_else(|e| panic!("{name} {format:?}: {e}"));
            for (i, (plane, buf)) in layout.iter().zip(&buffers).enumerate() {
                let out = buf.read();
                assert!(
                    out[plane.size()..].iter().all(|&b| b == POISON),
                    "{name} {format:?}: plane {i} written past its layout"
                );
                let untouched = out[..plane.size()].iter().filter(|&&b| b == POISON).count();
                assert!(
                    untouched < plane.size() / 8,
                    "{name} {format:?}: plane {i} not fully written ({untouched} bytes untouched)"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires an AMD GPU"]
fn crop() {
    let decoder = Decoder::new(Backend::Hardware, 0).unwrap();
    let data = sample("mug_420.jpg").expect("mug_420.jpg not found");
    let stream = Stream::parse(&data).unwrap();
    let (w, h) = (64usize, 48usize);

    let buffer = DeviceBuffer::new(w * 3 * h);
    let mut image = Image::new();
    image.set_plane(0, buffer.ptr.cast(), (w * 3) as u32);
    let mut params = DecodeParams::new(OutputFormat::Rgb);
    params.crop = Some(Rect {
        left: 0,
        top: 0,
        right: w as i16,
        bottom: h as i16,
    });
    unsafe { decoder.decode(&stream, &params, &mut image) }.unwrap();
    let out = buffer.read();
    assert!(out[w * 3 * h..].iter().all(|&b| b == POISON));
    assert!(out[..w * 3 * h].iter().any(|&b| b != POISON));
}

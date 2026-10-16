// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! Decode throughput of `rocJpegDecode`: `bench <file.jpg> [iterations]`.

use std::ffi::c_void;
use std::os::raw::c_int;
use std::time::Instant;

use rocjpeg::{Backend, DecodeParams, Decoder, Image, OutputFormat, Stream};

#[link(name = "amdhip64")]
unsafe extern "C" {
    fn hipMalloc(ptr: *mut *mut c_void, size: usize) -> c_int;
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: bench <file.jpg> [iterations]");
    let iterations: usize = std::env::args().nth(2).map_or(200, |s| s.parse().unwrap());
    let data = std::fs::read(&path).unwrap();

    let decoder = Decoder::new(Backend::Hardware, 0).unwrap();
    let stream = Stream::parse(&data).unwrap();
    let info = decoder.image_info(&stream).unwrap();
    println!(
        "{path}: {}x{} {:?}, {:.1} KiB",
        info.width(),
        info.height(),
        info.subsampling,
        data.len() as f64 / 1024.0
    );

    for format in [OutputFormat::Native, OutputFormat::YuvPlanar, OutputFormat::Rgb] {
        let Some(layout) = info.plane_layout(format) else { continue };
        let mut image = Image::new();
        for (i, plane) in layout.iter().enumerate() {
            let mut ptr = std::ptr::null_mut();
            assert_eq!(unsafe { hipMalloc(&mut ptr, plane.size()) }, 0);
            image.set_plane(i, ptr.cast(), plane.row_bytes as u32);
        }
        let params = DecodeParams::new(format);
        // Warm up (surface pool, interop mappings).
        for _ in 0..10 {
            unsafe { decoder.decode(&stream, &params, &mut image) }.unwrap();
        }
        let mut times = Vec::with_capacity(iterations);
        for _ in 0..iterations {
            let start = Instant::now();
            unsafe { decoder.decode(&stream, &params, &mut image) }.unwrap();
            times.push(start.elapsed().as_secs_f64() * 1000.0);
        }
        times.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let mean = times.iter().sum::<f64>() / times.len() as f64;
        println!(
            "{format:?}: mean {mean:.2} ms ({:.0} fps), median {:.2} ms, p95 {:.2} ms, min {:.2} ms",
            1000.0 / mean,
            times[times.len() / 2],
            times[times.len() * 95 / 100],
            times[0]
        );
    }
}

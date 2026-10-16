// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! Decodes a JPEG to RGB on the GPU and checks whether the output is complete as soon as
//! `rocJpegDecode` returns (i.e. without any extra HIP synchronization).

use std::ffi::c_void;
use std::os::raw::{c_int, c_uint};

use rocjpeg_sys::*;

#[link(name = "amdhip64")]
unsafe extern "C" {
    fn hipMalloc(ptr: *mut *mut c_void, size: usize) -> c_int;
    fn hipMemset(ptr: *mut c_void, value: c_int, size: usize) -> c_int;
    fn hipMemcpy(dst: *mut c_void, src: *const c_void, size: usize, kind: c_uint) -> c_int;
    fn hipDeviceSynchronize() -> c_int;
    fn hipFree(ptr: *mut c_void) -> c_int;
}
const HIP_MEMCPY_DEVICE_TO_HOST: c_uint = 2;

fn check(status: RocJpegStatus, what: &str) {
    if status != RocJpegStatus::ROCJPEG_STATUS_SUCCESS {
        let name = unsafe { std::ffi::CStr::from_ptr(rocJpegGetErrorName(status)) };
        panic!("{what}: {name:?}");
    }
}

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: decode <file.jpg> [iterations]");
    let iterations: usize = std::env::args().nth(2).map_or(20, |s| s.parse().unwrap());
    let data = std::fs::read(&path).unwrap();

    unsafe {
        let mut handle: RocJpegHandle = std::ptr::null_mut();
        check(
            rocJpegCreate(RocJpegBackend::ROCJPEG_BACKEND_HARDWARE, 0, &mut handle),
            "rocJpegCreate",
        );
        let mut stream: RocJpegStreamHandle = std::ptr::null_mut();
        check(rocJpegStreamCreate(&mut stream), "rocJpegStreamCreate");
        check(
            rocJpegStreamParse(data.as_ptr(), data.len(), stream),
            "rocJpegStreamParse",
        );

        let mut num_components = 0u8;
        let mut subsampling = RocJpegChromaSubsampling::ROCJPEG_CSS_UNKNOWN;
        let mut widths = [0u32; 4];
        let mut heights = [0u32; 4];
        check(
            rocJpegGetImageInfo(
                handle,
                stream,
                &mut num_components,
                &mut subsampling,
                widths.as_mut_ptr(),
                heights.as_mut_ptr(),
            ),
            "rocJpegGetImageInfo",
        );
        let (w, h) = (widths[0] as usize, heights[0] as usize);
        println!("{path}: {w}x{h}, {num_components} components, {subsampling:?}");

        let pitch = w * 3;
        let size = pitch * h;
        let mut dev: *mut c_void = std::ptr::null_mut();
        assert_eq!(hipMalloc(&mut dev, size), 0);

        let mut image: RocJpegImage = std::mem::zeroed();
        image.channel[0] = dev as *mut u8;
        image.pitch[0] = pitch as u32;
        let params = RocJpegDecodeParams {
            output_format: RocJpegOutputFormat::ROCJPEG_OUTPUT_RGB,
            ..std::mem::zeroed()
        };

        let mut reference = vec![0u8; size];
        let mut racy = 0;
        for i in 0..=iterations {
            // Poison the output so that an incomplete decode is detectable.
            assert_eq!(
                hipMemset(dev, if i % 2 == 0 { 0xA5 } else { 0x5A }, size),
                0
            );
            assert_eq!(hipDeviceSynchronize(), 0);

            check(
                rocJpegDecode(handle, stream, &params, &mut image),
                "rocJpegDecode",
            );

            // Read back immediately, without hipDeviceSynchronize.
            let mut out = vec![0u8; size];
            assert_eq!(
                hipMemcpy(
                    out.as_mut_ptr().cast(),
                    dev,
                    size,
                    HIP_MEMCPY_DEVICE_TO_HOST
                ),
                0
            );

            // Reference: read back again after a full device sync.
            assert_eq!(hipDeviceSynchronize(), 0);
            let mut synced = vec![0u8; size];
            assert_eq!(
                hipMemcpy(
                    synced.as_mut_ptr().cast(),
                    dev,
                    size,
                    HIP_MEMCPY_DEVICE_TO_HOST
                ),
                0
            );

            if i == 0 {
                reference = synced.clone();
            }
            assert_eq!(
                synced, reference,
                "decode result differs between iterations"
            );
            if out != synced {
                racy += 1;
            }
        }
        println!("pixel[0..6] = {:?}", &reference[..6]);
        println!("incomplete-at-return: {racy}/{} iterations", iterations + 1);

        hipFree(dev);
        rocJpegStreamDestroy(stream);
        rocJpegDestroy(handle);
    }
}

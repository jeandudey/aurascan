// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! These tests need an AMD GPU with a VCN JPEG engine, the `hip` GStreamer plugin and the
//! sample images shipped by the rocjpeg package (`$CONDA_PREFIX/share/rocjpeg/images`). Run
//! them with `cargo test -p gst-plugin-rocm -- --ignored`.

use gst::prelude::*;

fn init() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        gst::init().unwrap();
        gstrocm::plugin_register_static().unwrap();
    });
}

fn sample(name: &str) -> String {
    let prefix = std::env::var("CONDA_PREFIX").expect("CONDA_PREFIX is not set");
    format!("{prefix}/share/rocjpeg/images/{name}")
}

/// Decodes `image` with `rocjpegdec` into HIP memory, downloads it and returns the RGB frame
/// and its size.
fn decode_rgb(image: &str, src_caps: &str) -> (Vec<u8>, i32, i32) {
    init();
    let pipeline = gst::parse::launch(&format!(
        "filesrc location={image} ! jpegparse ! rocjpegdec ! {src_caps} ! hipdownload ! \
         videoconvert ! video/x-raw,format=RGB ! appsink name=sink"
    ))
    .unwrap()
    .downcast::<gst::Pipeline>()
    .unwrap();
    let sink = pipeline
        .by_name("sink")
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();

    pipeline.set_state(gst::State::Playing).unwrap();
    let sample = sink.pull_sample().expect("no frame decoded");
    let info = gst_video::VideoInfo::from_caps(sample.caps().unwrap()).unwrap();
    let frame = {
        let buffer = sample.buffer().unwrap();
        let map = buffer.map_readable().unwrap();
        map.as_slice().to_vec()
    };
    pipeline.set_state(gst::State::Null).unwrap();
    (frame, info.width() as i32, info.height() as i32)
}

fn cpu_rgb(image: &str) -> Vec<u8> {
    init();
    let pipeline = gst::parse::launch(&format!(
        "filesrc location={image} ! jpegparse ! jpegdec ! videoconvert ! \
         video/x-raw,format=RGB ! appsink name=sink"
    ))
    .unwrap()
    .downcast::<gst::Pipeline>()
    .unwrap();
    let sink = pipeline
        .by_name("sink")
        .unwrap()
        .downcast::<gst_app::AppSink>()
        .unwrap();
    pipeline.set_state(gst::State::Playing).unwrap();
    let sample = sink.pull_sample().unwrap();
    let frame = sample.buffer().unwrap().map_readable().unwrap().to_vec();
    pipeline.set_state(gst::State::Null).unwrap();
    frame
}

fn mean_abs_diff(a: &[u8], b: &[u8]) -> f64 {
    assert_eq!(a.len(), b.len());
    a.iter()
        .zip(b)
        .map(|(&a, &b)| (a as f64 - b as f64).abs())
        .sum::<f64>()
        / a.len() as f64
}

#[test]
#[ignore = "requires an AMD GPU"]
fn yuv_output_matches_libjpeg() {
    for (image, caps) in [
        ("mug_420.jpg", "video/x-raw(memory:HIPMemory),format=NV12"),
        ("mug_420.jpg", "video/x-raw(memory:HIPMemory),format=I420"),
        ("mug_422.jpg", "video/x-raw(memory:HIPMemory),format=Y42B"),
    ] {
        let path = sample(image);
        let (frame, width, height) = decode_rgb(&path, caps);
        assert_eq!((width, height), (3840, 2160));
        let diff = mean_abs_diff(&frame, &cpu_rgb(&path));
        assert!(diff < 2.0, "{image} {caps}: mean abs difference {diff}");
    }
}

#[test]
#[ignore = "requires an AMD GPU"]
fn grayscale_is_output_as_rgb() {
    let path = sample("mug_400.jpg");
    let (frame, width, height) = decode_rgb(&path, "video/x-raw(memory:HIPMemory)");
    assert_eq!((width, height), (3840, 2160));
    let diff = mean_abs_diff(&frame, &cpu_rgb(&path));
    assert!(diff < 2.0, "mean abs difference {diff}");
}

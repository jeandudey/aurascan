// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MPL-2.0

//! Emulates a live MJPEG webcam with an `appsrc` so that display problems seen with
//! `v4l2src ! jpegparse ! rocjpegdec ! hipdownload name=dl ! videoconvert ! autovideosink` can be
//! reproduced (and minimised) without the camera.
//!
//! The frames are read from a directory of JPEG files (for instance frames captured from the
//! camera with `multifilesink`). Exits with status 1 when `hipdownload` reports that the
//! HIP to GL interop copy failed.
//!
//! ```text
//! cargo run -p gst-plugin-rocm --example livecam_repro -- DIR [--delay-ms N] [--burst N]
//!     [--fps N] [--seconds N] [--decoder rocjpegdec|cpu] [--sink autovideosink]
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use gst::prelude::*;

struct Args {
    dir: String,
    delay_ms: u64,
    burst: usize,
    fps: u64,
    seconds: u64,
    decoder: String,
    sink: String,
    /// Post a LATENCY message after the first frame and recalculate the pipeline latency like
    /// `gst-launch` does (`v4l2src` does this).
    latency: bool,
    /// Send an upstream RECONFIGURE from the sink this many ms after the first frame.
    reconfigure_ms: Option<u64>,
}

fn parse_args() -> Args {
    let mut args = Args {
        dir: String::new(),
        delay_ms: 1200,
        burst: 4,
        fps: 60,
        seconds: 3,
        decoder: "rocjpegdec".into(),
        sink: "autovideosink sync=false".into(),
        latency: true,
        reconfigure_ms: None,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        let mut value = || it.next().expect("missing value");
        match arg.as_str() {
            "--delay-ms" => args.delay_ms = value().parse().unwrap(),
            "--burst" => args.burst = value().parse().unwrap(),
            "--fps" => args.fps = value().parse().unwrap(),
            "--seconds" => args.seconds = value().parse().unwrap(),
            "--decoder" => args.decoder = value(),
            "--sink" => args.sink = value(),
            "--no-latency" => args.latency = false,
            "--reconfigure-ms" => args.reconfigure_ms = Some(value().parse().unwrap()),
            _ => args.dir = arg,
        }
    }
    assert!(!args.dir.is_empty(), "usage: livecam_repro DIR [options]");
    args
}

fn main() {
    let args = parse_args();
    gst::init().unwrap();
    gstrocm::plugin_register_static().unwrap();

    // Detect the failure through the debug log.
    let failed = Arc::new(AtomicBool::new(false));
    let pools = Arc::new(std::sync::Mutex::new(std::collections::BTreeSet::<String>::new()));
    gst::log::set_threshold_for_name("hipmemorycopy", gst::DebugLevel::Warning);
    gst::log::set_threshold_for_name("bufferpool", gst::DebugLevel::Debug);
    let log_function = {
        let failed = failed.clone();
        let pools = pools.clone();
        gst::log::add_log_function(move |cat, level, _, _, _, object, message| {
            if cat.name() == "bufferpool" {
                if let Some(object) = object {
                    let name = object.to_string();
                    if let Some(pos) = name.find("bufferpool") {
                        let start = name[..pos]
                            .rfind(|c: char| !c.is_alphanumeric())
                            .map_or(0, |i| i + 1);
                        let tail: String = name[start..]
                            .chars()
                            .take_while(|c| c.is_alphanumeric())
                            .collect();
                        if tail.ends_with(char::is_numeric) {
                            pools.lock().unwrap().insert(tail);
                        }
                    }
                }
            }
            if cat.name() == "hipmemorycopy" && level <= gst::DebugLevel::Warning {
                if let Some(text) = message.get() {
                    if text.contains("fallback to system copy") {
                        failed.store(true, Ordering::SeqCst);
                    }
                }
            }
        })
    };
    gst::log::remove_default_log_function();

    let decoder = match args.decoder.as_str() {
        "cpu" => "jpegdec ! hipupload ! video/x-raw(memory:HIPMemory)",
        other => other,
    };
    let pipeline = gst::parse::launch(&format!(
        "appsrc name=src is-live=true format=time do-timestamp=true \
         caps=image/jpeg,width=1920,height=1080,framerate={}/1 ! jpegparse ! {decoder} ! \
         hipdownload ! videoconvert ! {} name=vs",
        args.fps, args.sink
    ))
    .unwrap()
    .downcast::<gst::Pipeline>()
    .unwrap();
    let src = pipeline
        .by_name("src")
        .unwrap()
        .downcast::<gst_app::AppSrc>()
        .unwrap();

    let mut frames: Vec<Vec<u8>> = std::fs::read_dir(&args.dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "jpg"))
        .collect::<Vec<_>>()
        .into_iter()
        .map(|p| std::fs::read(p).unwrap())
        .collect();
    assert!(!frames.is_empty(), "no jpg files in {}", args.dir);
    frames.shrink_to_fit();

    let reconfigures = Arc::new(std::sync::Mutex::new(Vec::<u64>::new()));
    let start = std::time::Instant::now();
    src.static_pad("src").unwrap().add_probe(
        gst::PadProbeType::EVENT_UPSTREAM,
        {
            let reconfigures = reconfigures.clone();
            move |_, info| {
                if let Some(gst::PadProbeData::Event(event)) = &info.data {
                    if event.type_() == gst::EventType::Reconfigure {
                        reconfigures
                            .lock()
                            .unwrap()
                            .push(start.elapsed().as_millis() as u64);
                    }
                }
                gst::PadProbeReturn::Ok
            }
        },
    );

    pipeline.set_state(gst::State::Playing).unwrap();

    let delay = Duration::from_millis(args.delay_ms);
    let interval = Duration::from_micros(1_000_000 / args.fps);
    let total = (args.seconds * args.fps) as usize;
    let latency = args.latency;
    let reconfigure_ms = args.reconfigure_ms;
    let dl_sink = pipeline.by_name("vs").unwrap().static_pad("sink").unwrap();
    let pusher = std::thread::spawn({
        let src = src.clone();
        move || {
            // The camera needs a while before the first frame, then delivers the frames it
            // queued in a burst.
            std::thread::sleep(delay);
            for i in 0..total {
                let data = frames[i % frames.len()].clone();
                if src.push_buffer(gst::Buffer::from_slice(data)).is_err() {
                    return;
                }
                if i == 0 {
                    if let Some(ms) = reconfigure_ms {
                        let dl_sink = dl_sink.clone();
                        std::thread::spawn(move || {
                            std::thread::sleep(Duration::from_millis(ms));
                            dl_sink.push_event(gst::event::Reconfigure::new());
                        });
                    }
                }
                if i == 0 && latency {
                    let _ = src.post_message(gst::message::Latency::builder().src(&src).build());
                }
                if i + 1 >= args.burst {
                    std::thread::sleep(interval);
                }
            }
            let _ = src.end_of_stream();
        }
    });

    let bus = pipeline.bus().unwrap();
    let mut error = None;
    for msg in bus.iter_timed(gst::ClockTime::from_seconds(args.seconds + 10)) {
        use gst::MessageView;
        match msg.view() {
            MessageView::Eos(..) => break,
            MessageView::Latency(..) => {
                let _ = pipeline.recalculate_latency();
            }
            MessageView::Error(err) => {
                error = Some(err.error().to_string());
                break;
            }
            _ => {}
        }
    }
    pipeline.set_state(gst::State::Null).unwrap();
    let _ = pusher.join();
    gst::log::remove_log_function(log_function);

    if let Some(error) = error {
        println!("ERROR {error}");
        std::process::exit(2);
    }
    println!("reconfigure events at ms: {:?}", reconfigures.lock().unwrap());
    let pools = pools.lock().unwrap().iter().cloned().collect::<Vec<_>>().join(",");
    if failed.load(Ordering::SeqCst) {
        println!("pools {pools}");
        println!("FAIL");
        std::process::exit(1);
    }
    println!("pools {pools}");
    println!("ok");
}

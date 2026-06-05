use crate::face_tracker::FaceTracker;
use gst::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use gst_analytics::prelude::*;
use gstburnextra::BackendType;
use gstburnextra::scrfd::ModelType;
use gtk::Application;
use gtk::glib;
use gtk::prelude::*;
use std::sync::Mutex;

mod face_tracker;

const FACE_CLASS_LABEL: &glib::GStr = glib::gstr!("face");

fn main() -> glib::ExitCode {
    gst::init().unwrap();
    gtk::init().unwrap();

    gstgtk4::plugin_register_static().unwrap();
    gstburnextra::plugin_register_static().unwrap();

    let app = Application::builder()
        .application_id("tech.jeandudey.PoseKit")
        .build();

    app.connect_activate(activate);
    let res = app.run();
    unsafe { gst::deinit() };
    res
}

fn activate(app: &gtk::Application) {
    let pipeline = gst::Pipeline::new();

    let caps0 = gst::Caps::builder("video/x-raw")
        .field("format", "RGB")
        .field("width", 640)
        .field("height", 640)
        .build();
    let caps1 = gst::Caps::builder("video/x-raw")
        .field("format", "RGB")
        .field("width", 224)
        .field("height", 224)
        .build();

    let src = gst::ElementFactory::make("v4l2src").build().unwrap();
    let videoconvert0 = gst::ElementFactory::make("videoconvert").build().unwrap();
    let videoscale0 = gst::ElementFactory::make("videoscale").build().unwrap();
    let videoscale1 = gst::ElementFactory::make("videoscale").build().unwrap();
    let capsfilter0 = gst::ElementFactory::make("capsfilter")
        .property("caps", &caps0)
        .build()
        .unwrap();
    let capsfilter1 = gst::ElementFactory::make("capsfilter")
        .property("caps", &caps1)
        .build()
        .unwrap();
    let tee = gst::ElementFactory::make("tee").build().unwrap();
    let queue0 = gst::ElementFactory::make("queue").build().unwrap();
    let queue1 = gst::ElementFactory::make("queue").build().unwrap();
    let scrfdinference = gst::ElementFactory::make("burnextra-scrfdinference")
        .property("model-type", ModelType::Scrfd500m)
        .property("backend-type", BackendType::Vulkan)
        .build()
        .unwrap();
    let scrfdtensordec = gst::ElementFactory::make("scrfdtensordec").build().unwrap();
    let tracker = gst::ElementFactory::make("bytetracker").build().unwrap();
    let sixdrepnet360inference = gst::ElementFactory::make("burnextra-sixdrepnet360inference")
        .property("backend-type", BackendType::Vulkan)
        .build()
        .unwrap();
    let sixdrepnet360tensordec = gst::ElementFactory::make("sixdrepnet360tensordec")
        .build()
        .unwrap();
    let videocrop = gst::ElementFactory::make("videocrop").build().unwrap();
    let sink0 = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .unwrap();
    let sink1 = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .unwrap();

    pipeline
        .add_many([
            &src,
            &videoconvert0,
            &videoscale0,
            &videoscale1,
            &capsfilter0,
            &capsfilter1,
            &tee,
            &queue0,
            &queue1,
            &scrfdinference,
            &scrfdtensordec,
            &tracker,
            &sixdrepnet360inference,
            &sixdrepnet360tensordec,
            &videocrop,
            &sink0,
            &sink1,
        ])
        .unwrap();

    gst::Element::link_many([
        &src,
        &videoconvert0,
        &videoscale0,
        &capsfilter0,
        &scrfdinference,
        &scrfdtensordec,
        &tracker,
        &tee,
    ])
    .unwrap();

    let t_display = tee.request_pad_simple("src_%u").unwrap();
    let q_display = queue0.static_pad("sink").unwrap();
    t_display.link(&q_display).unwrap();
    gst::Element::link_many([&queue0, &sink0]).unwrap();

    let t_crop = tee.request_pad_simple("src_%u").unwrap();
    let q_crop = queue1.static_pad("sink").unwrap();
    t_crop.link(&q_crop).unwrap();

    gst::Element::link_many([
        &queue1,
        &videocrop,
        &videoscale1,
        &capsfilter1,
        &sixdrepnet360inference,
        &sixdrepnet360tensordec,
        &sink1,
    ])
    .unwrap();

    let tracker = Mutex::new(FaceTracker::new());
    let last_ts = Mutex::new(None::<gst::ClockTime>);

    let videocrop_sink = videocrop.static_pad("sink").unwrap();
    videocrop_sink.add_probe(
        gst::PadProbeType::BUFFER,
        glib::clone!(
            #[strong]
            videocrop,
            move |pad, info| {
                let Some(buffer) = info.buffer() else {
                    return gst::PadProbeReturn::Ok;
                };

                let dt = {
                    let mut lt = last_ts.lock().unwrap();
                    let pts = buffer.pts();
                    let dt = match (*lt, pts) {
                        (Some(prev), Some(now)) => now.saturating_sub(prev).seconds_f32(),
                        _ => 1.0 / 30.0,
                    };
                    *lt = pts;
                    dt.max(1e-3)
                };

                let class = glib::Quark::from_static_str(FACE_CLASS_LABEL);

                let mut detections = Vec::new();
                for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
                    for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
                        if let Some(ty) = od.obj_type() {
                            if ty != class {
                                continue;
                            }
                        } else {
                            continue;
                        }

                        let location = od.location().unwrap();
                        detections.push((
                            location.x as f32 + location.w as f32 / 2.0,
                            location.y as f32 + location.h as f32 / 2.0,
                            location.w as f32,
                            location.h as f32,
                        ));
                    }
                }

                let (frame_w, frame_h) = match pad
                    .current_caps()
                    .and_then(|caps| gst_video::VideoInfo::from_caps(&caps).ok())
                {
                    Some(info) => (info.width() as i32, info.height() as i32),
                    None => return gst::PadProbeReturn::Ok, // not negotiated yet
                };

                // cx,cy is the stable tracked center.
                if let Some((cx, cy, w, h)) = tracker.lock().unwrap().step(&detections, dt) {
                    let (top, bottom, left, right) =
                        crop_props(cx, cy, w, h, frame_w as f32, frame_h as f32);
                    videocrop.set_property("top", top);
                    videocrop.set_property("bottom", bottom);
                    videocrop.set_property("left", left);
                    videocrop.set_property("right", right);
                }

                gst::PadProbeReturn::Ok
            }
        ),
    );

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let hbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .build();
    vbox.append(&hbox);

    let sink0_picture = gstgtk4::RenderWidget::new(&sink0);
    hbox.append(&sink0_picture);

    let sink1_picture = gstgtk4::RenderWidget::new(&sink1);
    hbox.append(&sink1_picture);

    let button = gtk::Button::builder().label("Start").build();
    vbox.append(&button);

    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("PoseKit")
        .default_width(1280)
        .default_height(720)
        .child(&vbox)
        .build();

    let pipeline_clone = pipeline.clone();

    window.connect_close_request(move |_| {
        let _ = pipeline_clone.set_state(gst::State::Null);
        glib::Propagation::Proceed
    });

    pipeline.set_state(gst::State::Playing).unwrap();

    window.present();
}

fn crop_props(
    cx: f32,
    cy: f32,
    w: f32,
    h: f32, // tracked face box center + size
    frame_w: f32,
    frame_h: f32,
) -> (i32, i32, i32, i32) {
    // asymmetric expansion
    let exp_top = 0.45;
    let exp_bottom = 0.15;
    let exp_side = 0.25;

    let x0 = cx - w / 2.0;
    let y0 = cy - h / 2.0;

    let left = x0 - w * exp_side;
    let right = x0 + w + w * exp_side;
    let top = y0 - h * exp_top;
    let bottom = y0 + h + h * exp_bottom;

    // square it: take larger side, recenter both dims
    let bw = right - left;
    let bh = bottom - top;
    let side = bw.max(bh);
    let ccx = (left + right) / 2.0;
    let ccy = (top + bottom) / 2.0;

    let mut left = ccx - side / 2.0;
    let mut right = ccx + side / 2.0;
    let mut top = ccy - side / 2.0;
    let mut bottom = ccy + side / 2.0;

    // shift inward instead of truncating, to preserve square-ness near edges
    if left < 0.0 {
        right -= left;
        left = 0.0;
    }
    if top < 0.0 {
        bottom -= top;
        top = 0.0;
    }
    if right > frame_w {
        left -= right - frame_w;
        right = frame_w;
    }
    if bottom > frame_h {
        top -= bottom - frame_h;
        bottom = frame_h;
    }

    // final clamp in case the box is larger than the frame in some dim
    let left = left.max(0.0);
    let top = top.max(0.0);
    let right = right.min(frame_w);
    let bottom = bottom.min(frame_h);

    (
        top.round() as i32,                // top
        (frame_h - bottom).round() as i32, // bottom
        left.round() as i32,               // left
        (frame_w - right).round() as i32,  // right
    )
}

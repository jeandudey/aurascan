use burn::{Dispatch, DispatchDevice};
use gst::prelude::*;
use gst_video::prelude::*;
use gst_video::{VideoFrameRef, VideoInfo};
use gtk::Application;
use gtk::glib;
use gtk::prelude::*;
use image::{DynamicImage, RgbImage};
use scrfd_burn::Face;
use scrfd_burn::scrfd_500m::Model as Scrfd500M;
use std::cell::RefCell;
use std::rc::Rc;
use std::thread;

//mod face_analysis;

struct DetectionInfo {
    pub source_width: u32,
    pub source_height: u32,
    pub faces: Vec<Face>,
}

fn main() -> glib::ExitCode {
    gst::init().unwrap();
    gtk::init().unwrap();

    gstgtk4::plugin_register_static().unwrap();

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

    let src = gst::ElementFactory::make("v4l2src")
        .name("camera")
        .build()
        .unwrap();

    let videoconvert = gst::ElementFactory::make("videoconvert").build().unwrap();

    let tee = gst::ElementFactory::make("tee").build().unwrap();
    let queue_display = gst::ElementFactory::make("queue").build().unwrap();
    let queue_cv = gst::ElementFactory::make("queue")
        .property("max-size-buffers", 1u32)
        .property("max-size-bytes", 0u32)
        .property("max-size-time", 0u64)
        .property_from_str("leaky", "downstream")
        .build()
        .unwrap();

    let sink = gst::ElementFactory::make("gtk4paintablesink")
        .build()
        .unwrap();

    let appsink = gst::ElementFactory::make("appsink").build().unwrap();

    pipeline
        .add_many([
            &src,
            &videoconvert,
            &tee,
            &queue_display,
            &sink,
            &queue_cv,
            &appsink,
        ])
        .unwrap();

    gst::Element::link_many([&src, &videoconvert, &tee]).unwrap();

    let t_display = tee.request_pad_simple("src_%u").unwrap();
    let q_display = queue_display.static_pad("sink").unwrap();
    t_display.link(&q_display).unwrap();

    let t_cv = tee.request_pad_simple("src_%u").unwrap();
    let q_cv = queue_cv.static_pad("sink").unwrap();
    t_cv.link(&q_cv).unwrap();

    gst::Element::link_many([&queue_display, &sink]).unwrap();
    gst::Element::link_many([&queue_cv, &appsink]).unwrap();

    let appsink = appsink.downcast::<gst_app::AppSink>().unwrap();

    appsink.set_drop(true);
    appsink.set_max_buffers(1);

    let (sample_tx, sample_rx) = crossbeam_channel::bounded::<gst::Sample>(1);
    let (det_tx, det_rx) = async_channel::unbounded::<DetectionInfo>();
    appsink.set_callbacks(
        gst_app::AppSinkCallbacks::builder()
            .new_sample(move |sink| {
                let sample = sink.pull_sample().unwrap();
                sample_tx.try_send(sample).ok();
                Ok(gst::FlowSuccess::Ok)
            })
            .build(),
    );
    appsink.set_sync(false);
    appsink.set_drop(true);
    appsink.set_max_buffers(1);

    thread::Builder::new()
        .name("model".to_string())
        .spawn(move || {
            let device = DispatchDevice::Rocm(Default::default());
            let scrfd_500m = Scrfd500M::<Dispatch>::from_embedded(&device);

            while let Ok(sample) = sample_rx.recv() {
                let buffer = sample.buffer().unwrap();
                let caps = sample.caps().unwrap();
                let info = VideoInfo::from_caps(caps).unwrap();
                let frame = VideoFrameRef::from_buffer_ref_readable(buffer, &info).unwrap();

                let width = frame.width();
                let height = frame.height();
                let stride = info.stride()[0] as usize;
                let data = frame.plane_data(0).unwrap();
                let pixel_stride = info.format_info().pixel_stride()[0] as usize;

                let pixels: Vec<u8> = (0..height as usize)
                    .flat_map(|y| {
                        let row = &data[y * stride..y * stride + width as usize * pixel_stride];
                        (0..width as usize).flat_map(move |x| {
                            let p = x * pixel_stride;
                            [row[p], row[p + 1], row[p + 2]]
                        })
                    })
                    .collect();

                let image: DynamicImage = RgbImage::from_raw(width, height, pixels).unwrap().into();
                let faces = scrfd_500m.detect_image(image, 0.5, 0.4);
                println!("{faces:?}");
                det_tx
                    .send_blocking(DetectionInfo {
                        source_width: width,
                        source_height: height,
                        faces,
                    })
                    .ok();
            }
        })
        .unwrap();

    let vbox = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .build();

    let overlay = gtk::Overlay::builder().hexpand(true).vexpand(true).build();
    vbox.append(&overlay);

    let picture = gstgtk4::RenderWidget::new(&sink);
    overlay.set_child(Some(&picture));

    let draw_area = gtk::DrawingArea::builder().can_target(false).build();
    overlay.add_overlay(&draw_area);

    let faces = Rc::new(RefCell::new(DetectionInfo {
        source_width: 0,
        source_height: 0,
        faces: Vec::new(),
    }));
    draw_area.set_draw_func({
        let faces = Rc::clone(&faces);
        move |_, cr, w, h| {
            let info = faces.borrow();
            if info.faces.is_empty() {
                return;
            }

            let (w, h) = (w as f64, h as f64);
            let (sx, sy) = (w / info.source_width as f64, h / info.source_height as f64);

            cr.set_source_rgb(1.0, 0.0, 0.0);
            cr.set_line_width(2.0);
            for face in info.faces.iter() {
                cr.rectangle(
                    face.x1 as f64 * sx,
                    face.y1 as f64 * sy,
                    face.width() as f64 * sx,
                    face.height() as f64 * sy,
                );
                cr.stroke().unwrap();
            }
        }
    });

    glib::spawn_future_local(async move {
        while let Ok(mut latest_faces) = det_rx.recv().await {
            while let Ok(new_faces) = det_rx.try_recv() {
                latest_faces = new_faces;
            }
            *faces.borrow_mut() = latest_faces;
            draw_area.queue_draw();
        }
    });

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

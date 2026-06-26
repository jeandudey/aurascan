use std::sync::{LazyLock, Mutex};

use gst::glib;
use gst::glib::ParamSpecBuilderExt;
use gst::glib::value::ToValue;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use gst_base::subclass::prelude::BaseTransformImpl;

struct Settings {
    smoothing: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { smoothing: 0.3 }
    }
}

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "detectioncropmeta",
        gst::DebugColorFlags::empty(),
        Some("Detection Crop Metadata Element"),
    )
});

struct Selector {
    locked_id: Option<u64>,
    switch_margin: f32,
    grace_frames: u32,
    missing: u32,
}

impl Selector {
    fn pick(&mut self, dets: &[Detection], frame_w: f32, frame_h: f32) -> Option<u64> {
        let best = dets.iter().max_by(|a, b| {
            a.score(frame_w, frame_h)
                .partial_cmp(&b.score(frame_w, frame_h))
                .unwrap()
        });

        match self.locked_id {
            Some(id) => {
                if let Some(cur) = dets.iter().find(|d| d.track_id == id) {
                    self.missing = 0;
                    let cur_s = cur.score(frame_w, frame_h);
                    if let Some(best) = best {
                        if best.track_id != id
                            && best.score(frame_w, frame_h) > cur_s + self.switch_margin
                        {
                            self.locked_id = Some(best.track_id);
                        }
                    }
                } else {
                    self.missing += 1;
                    if self.missing > self.grace_frames {
                        self.locked_id = best.map(|d| d.track_id);
                        self.missing = 0;
                    }
                }
            }
            None => self.locked_id = best.map(|d| d.track_id),
        }
        self.locked_id
    }
}

struct Detection {
    track_id: u64,
    /// `[x, y, w, h]`
    bbox: [f32; 4],
    score: f32,
}

impl Detection {
    fn score(&self, frame_w: f32, frame_h: f32) -> f32 {
        let [x, y, w, h] = self.bbox;
        let area = (w * h) / (frame_w * frame_h);
        let cx = x + w * 0.5;
        let cy = y + h * 0.5;
        let dx = (cx - frame_w * 0.5) / frame_w;
        let dy = (cy - frame_h * 0.5) / frame_h;
        let centrality = 1.0 - (dx * dx + dy * dy).sqrt();
        0.6 * area + 0.3 * centrality + 0.1 * self.score
    }
}

/// Smooths the crop region over time with EMA.
///
/// Call `advance` whenever a fresh detection is available, then call `rect` to
/// get the crop to emit.  When no detection has ever been seen `rect` returns
/// the full frame so downstream always receives a valid region.
struct Cropper {
    cx: f32,
    cy: f32,
    side: f32,
    initialized: bool,
}

impl Cropper {
    fn new() -> Self {
        Self {
            cx: 0.0,
            cy: 0.0,
            side: 0.0,
            initialized: false,
        }
    }

    fn advance(&mut self, location: &gst_analytics::AnalyticsODLocation, alpha: f32) {
        let w = location.w as f32;
        let h = location.h as f32;
        let x = location.x as f32;
        let y = location.y as f32;

        let exp_top = 0.45;
        let exp_bottom = 0.15;
        let exp_side = 0.25;

        let left_raw = x - w * exp_side;
        let right_raw = x + w + w * exp_side;
        let top_raw = y - h * exp_top;
        let bottom_raw = y + h + h * exp_bottom;

        let bw = right_raw - left_raw;
        let bh = bottom_raw - top_raw;
        let t_side = bw.max(bh);
        let t_cx = (left_raw + right_raw) / 2.0;
        let t_cy = (top_raw + bottom_raw) / 2.0;

        if !self.initialized {
            self.cx = t_cx;
            self.cy = t_cy;
            self.side = t_side;
            self.initialized = true;
        } else {
            self.cx += alpha * (t_cx - self.cx);
            self.cy += alpha * (t_cy - self.cy);
            self.side += alpha * (t_side - self.side);
        }
    }

    /// Returns `(x, y, width, height)` for `GstVideoCropMeta`.
    ///
    /// Falls back to the full frame when no detection has been seen yet so that
    /// downstream always gets a valid region even before the tracker fires.
    fn rect(&self, frame_w: f32, frame_h: f32) -> (u32, u32, u32, u32) {
        if !self.initialized {
            return (0, 0, frame_w as u32, frame_h as u32);
        }

        let side = self.side.min(frame_w - 1.0).min(frame_h - 1.0).max(1.0);
        let mut left = self.cx - side / 2.0;
        let mut top = self.cy - side / 2.0;
        let mut right = left + side;
        let mut bottom = top + side;

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

        let left = left.max(0.0);
        let top = top.max(0.0);
        let right = right.min(frame_w);
        let bottom = bottom.min(frame_h);

        let x = left.round() as u32;
        let y = top.round() as u32;
        let width = (right - left).round().max(0.0) as u32;
        let height = (bottom - top).round().max(0.0) as u32;

        (x, y, width, height)
    }
}

struct TrackerState {
    selector: Selector,
    cropper: Cropper,
}

#[derive(Default)]
pub struct DetectionCropMeta {
    settings: Mutex<Settings>,
    tracker_state: Mutex<Option<TrackerState>>,
    video_info: Mutex<Option<gst_video::VideoInfo>>,
}

#[glib::object_subclass]
impl ObjectSubclass for DetectionCropMeta {
    const NAME: &'static str = "GstAscDetectionCropMeta";

    type Type = super::DetectionCropMeta;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for DetectionCropMeta {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![glib::ParamSpecFloat::builder("smoothing")
                .nick("EMA Smoothing Factor")
                .blurb("Controls how quickly the crop region tracks the detected object (0 = frozen, 1 = instant)")
                .minimum(0.0)
                .maximum(1.0)
                .default_value(Settings::default().smoothing)
                .mutable_ready()
                .build()]
        });

        &*PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "smoothing" => {
                let mut settings = self.settings.lock().unwrap();
                settings.smoothing = value.get().unwrap();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "smoothing" => {
                let settings = self.settings.lock().unwrap();
                settings.smoothing.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for DetectionCropMeta {}

impl ElementImpl for DetectionCropMeta {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Detection Crop Metadata",
                "Filter/Video",
                "Selects the best tracked object detection and annotates the buffer with GstVideoCropMeta",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst::Caps::builder("video/x-raw").build();
            let sink = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();
            let src = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();
            vec![sink, src]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for DetectionCropMeta {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        *self.tracker_state.lock().unwrap() = Some(TrackerState {
            selector: Selector {
                locked_id: None,
                switch_margin: 0.15,
                grace_frames: 15,
                missing: 0,
            },
            cropper: Cropper::new(),
        });

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.tracker_state.lock().unwrap() = None;
        *self.video_info.lock().unwrap() = None;

        gst::info!(CAT, imp = self, "Stopped");

        Ok(())
    }

    fn set_caps(&self, incaps: &gst::Caps, _outcaps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let video_info = gst_video::VideoInfo::from_caps(incaps)
            .map_err(|_| gst::loggable_error!(CAT, "Failed to parse input caps"))?;
        *self.video_info.lock().unwrap() = Some(video_info);
        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut tracker_state_guard = self.tracker_state.lock().unwrap();
        let Some(tracker_state) = &mut *tracker_state_guard else {
            gst::error!(CAT, imp = self, "Wrong state");
            return Err(gst::FlowError::Flushing);
        };

        let video_info_guard = self.video_info.lock().unwrap();
        let Some(video_info) = &*video_info_guard else {
            return Ok(gst::FlowSuccess::Ok);
        };

        let frame_w = video_info.width() as f32;
        let frame_h = video_info.height() as f32;
        let smoothing = self.settings.lock().unwrap().smoothing;

        // Collect detections from analytics metadata (immutable borrow of buffer).
        let mut detections = Vec::new();
        let Some(meta) = buffer.meta::<gst_analytics::AnalyticsRelationMeta>() else {
            return Ok(gst::FlowSuccess::Ok);
        };

        for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
            let Ok(location) = od.location() else {
                continue;
            };

            let Some(track_id) = object_detection_tracking_id(&meta, &od) else {
                continue;
            };

            detections.push(Detection {
                track_id,
                bbox: [
                    location.x as f32,
                    location.y as f32,
                    location.w as f32,
                    location.h as f32,
                ],
                score: location.loc_conf_lvl as f32,
            });
        }

        let chosen_id = tracker_state.selector.pick(&detections, frame_w, frame_h);

        // When a detection is selected, advance the EMA toward its location.
        let location = chosen_id.and_then(|chosen_id| find_location(chosen_id, buffer));
        if let Some(location) = location.as_ref() {
            tracker_state.cropper.advance(&location, smoothing);
        }

        // Always emit a rect: last known smoothed position, or full frame when
        // no detection has ever been seen.
        let (x, y, width, height) = tracker_state.cropper.rect(frame_w, frame_h);
        drop(video_info_guard);

        gst::debug!(
            CAT,
            imp = self,
            "track_id={} x={x} y={y} width={width} height={height}",
            chosen_id.map_or(-1i64, |id| id as i64),
        );

        gst_video::VideoCropMeta::add(buffer, (x, y, width, height));

        if let Some(location) = location {
            let mut structure = gst::Structure::builder("selected-detection")
                .field("x", location.x)
                .field("y", location.y)
                .field("w", location.w)
                .field("h", location.h)
                .field("loc-conf-lvl", location.loc_conf_lvl);

            if let Some(id) = chosen_id {
                structure = structure.field("id", id);
            }

            if let Some(pts) = buffer.pts() {
                structure = structure.field("pts", pts);
            }

            self.obj()
                .post_message(gst::message::Application::new(structure.build()))
                .unwrap();
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

fn find_location(
    needed_id: u64,
    buffer: &gst::BufferRef,
) -> Option<gst_analytics::AnalyticsODLocation> {
    for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
        'od: for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
            let Ok(location) = od.location() else {
                continue 'od;
            };
            let Some(tr) = meta
                .iter_direct_related::<gst_analytics::AnalyticsTrackingMtd>(
                    od.id(),
                    gst_analytics::RelTypes::RELATE_TO,
                )
                .next()
            else {
                continue 'od;
            };
            let od_id = tr.info().0;
            if od_id != needed_id {
                continue 'od;
            }
            return Some(location);
        }
    }

    None
}

fn object_detection_tracking_id(
    meta: &gst::MetaRef<gst_analytics::AnalyticsRelationMeta>,
    object_detection: &gst_analytics::AnalyticsMtdRef<gst_analytics::AnalyticsODMtd>,
) -> Option<u64> {
    meta.iter_direct_related::<gst_analytics::AnalyticsTrackingMtd>(
        object_detection.id(),
        gst_analytics::RelTypes::RELATE_TO,
    )
    .next()
    .map(|tracking_meta| tracking_meta.info().0)
}

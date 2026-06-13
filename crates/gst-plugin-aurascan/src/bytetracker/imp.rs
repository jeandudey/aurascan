use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use gst::glib;
use gst::glib::ParamSpecBuilderExt;
use gst::glib::value::ToValue;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use gst_analytics::ffi::gst_analytics_relation_meta_add_tracking_mtd;
use gst_base::subclass::prelude::BaseTransformImpl;

use edgefirst_tracker::Tracker;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "bytetracker",
        gst::DebugColorFlags::empty(),
        Some("ByteTracker Object Tracking Element"),
    )
});

// TODO: Update settings.
struct Settings {
    track_high_conf: f32,
    track_iou: f32,
    track_update: f32,
    track_extra_lifespan: u64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            track_high_conf: 0.7,
            track_iou: 0.25,
            track_update: 0.25,
            track_extra_lifespan: 500_000_000, // 0.5 seconds
        }
    }
}

#[derive(Debug, Clone)]
struct DetectionBox {
    bbox: [f32; 4],
    score: f32,
}

impl edgefirst_tracker::DetectionBox for DetectionBox {
    fn bbox(&self) -> [f32; 4] {
        self.bbox
    }

    fn score(&self) -> f32 {
        self.score
    }

    fn label(&self) -> usize {
        0
    }
}

struct State {
    tracker: edgefirst_tracker::ByteTrack<DetectionBox>,
    uuid_map: HashMap<edgefirst_tracker::Uuid, u64>,
    next_id: u64,
}

impl State {
    fn new(settings: &Settings) -> Self {
        Self {
            tracker: edgefirst_tracker::ByteTrackBuilder::new()
                .track_extra_lifespan(settings.track_extra_lifespan)
                .track_high_conf(settings.track_high_conf)
                .track_iou(settings.track_iou)
                .track_update(settings.track_update)
                .build(),
            uuid_map: HashMap::new(),
            next_id: 0,
        }
    }
    fn id_for(&mut self, uuid: edgefirst_tracker::Uuid) -> u64 {
        let next = &mut self.next_id;
        *self.uuid_map.entry(uuid).or_insert_with(|| {
            let id = *next;
            *next += 1;
            id
        })
    }
}

#[derive(Default)]
pub struct ByteTracker {
    settings: Mutex<Settings>,
    state: Mutex<Option<State>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ByteTracker {
    const NAME: &'static str = "GstBurnExtraByteTracker";

    type Type = super::ByteTracker;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for ByteTracker {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecFloat::builder("track-high-conf")
                    .nick("High Confidence Threshold")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().track_high_conf)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecFloat::builder("track-iou")
                    .nick("IoU Treshold for Tracking")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().track_iou)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecFloat::builder("track-update")
                    .nick("Track Update Rate")
                    .blurb("Set the update rate for the kalman filter")
                    .minimum(0.0)
                    .default_value(Settings::default().track_update)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecULong::builder("track-extra-lifespan")
                    .nick("Extra Lifespan for Tracks")
                    .blurb("Set the extra lifespan for tracks in nanoseconds")
                    .default_value(Settings::default().track_extra_lifespan)
                    .mutable_ready()
                    .build(),
            ]
        });

        &*PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "track-high-conf" => {
                let mut settings = self.settings.lock().unwrap();
                settings.track_high_conf = value.get().unwrap();
            }
            "track-iou" => {
                let mut settings = self.settings.lock().unwrap();
                settings.track_iou = value.get().unwrap();
            }
            "track-update" => {
                let mut settings = self.settings.lock().unwrap();
                settings.track_update = value.get().unwrap();
            }
            "track-extra-lifespan" => {
                let mut settings = self.settings.lock().unwrap();
                settings.track_extra_lifespan = value.get().unwrap();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "track-high-conf" => {
                let settings = self.settings.lock().unwrap();
                settings.track_high_conf.to_value()
            }
            "track-iou" => {
                let settings = self.settings.lock().unwrap();
                settings.track_iou.to_value()
            }
            "track-update" => {
                let settings = self.settings.lock().unwrap();
                settings.track_update.to_value()
            }
            "track-extra-lifespan" => {
                let settings = self.settings.lock().unwrap();
                settings.track_extra_lifespan.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for ByteTracker {}

impl ElementImpl for ByteTracker {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "ByteTracker Object Tracking Element",
                "Analyzer/Video",
                "Track the objects across frames using the ByteTracker algorithm",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst::Caps::builder("video/x-raw").build();

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &caps,
            )
            .unwrap();

            vec![sink_pad_template, src_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for ByteTracker {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let settings = self.settings.lock().unwrap();
        let mut state = self.state.lock().unwrap();
        *state = Some(State::new(&settings));

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.state.lock().unwrap() = None;

        gst::info!(CAT, imp = self, "Stopped");

        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut state_guard = self.state.lock().unwrap();
        let Some(state) = &mut *state_guard else {
            gst::error!(CAT, imp = self, "Wrong state");
            return Err(gst::FlowError::Flushing);
        };

        let timestamp = buffer.pts().unwrap();
        let detections = detections(buffer);
        for (detection_index, track_info) in state
            .tracker
            .update(&detections, timestamp.nseconds())
            .into_iter()
            .enumerate()
            .filter_map(|(i, track_info)| Some((i, track_info?)))
        {
            set_track_id(
                buffer,
                detection_index,
                &track_info,
                state.id_for(track_info.uuid.clone()),
            );
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

fn set_track_id(
    buffer: &mut gst::BufferRef,
    detection_index: usize,
    track_info: &edgefirst_tracker::TrackInfo,
    track_id: u64,
) {
    let mut total_idx = 0;
    let mut meta_idx = 0;
    let mut od_id = None;
    'meta: for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
        for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
            if od.location().is_err() {
                gst::warning!(CAT, "Failed to get location from object detection metadata");
                continue;
            }

            if total_idx == detection_index {
                od_id = Some(od.id());
                break 'meta;
            } else {
                total_idx += 1;
            }
        }

        if total_idx >= detection_index {
            break 'meta;
        } else {
            meta_idx += 1;
        }
    }

    let Some(od_id) = od_id else {
        return;
    };

    let mut meta = buffer
        .iter_meta_mut::<gst_analytics::AnalyticsRelationMeta>()
        .nth(meta_idx)
        .expect("meta_idx should be valid");

    // TODO: This probably should be a safe method in gstreamer-analytics.
    //
    // This is how gstioutracker does it.
    let tracking_id = unsafe {
        let mut mtd = std::mem::MaybeUninit::uninit();
        gst_analytics_relation_meta_add_tracking_mtd(
            meta.as_mut_ptr(),
            track_id,
            track_info.created,
            mtd.as_mut_ptr(),
        );
        let mtd = mtd.assume_init();
        mtd.id
    };
    meta.set_relation(gst_analytics::RelTypes::RELATE_TO, od_id, tracking_id)
        .unwrap()
}

fn detections(buffer: &gst::BufferRef) -> Vec<DetectionBox> {
    let mut detections = Vec::new();
    for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
        for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
            let Ok(location) = od.location() else {
                gst::warning!(CAT, "Failed to get location from object detection metadata");
                continue;
            };
            detections.push(location_to_detection(&location));
        }
    }
    detections
}

fn location_to_detection(location: &gst_analytics::AnalyticsODLocation) -> DetectionBox {
    let score = location.loc_conf_lvl;
    let x = location.x as f32;
    let y = location.y as f32;
    let w = location.w as f32;
    let h = location.h as f32;
    DetectionBox {
        score,
        bbox: [x, y, x + w, y + h],
    }
}

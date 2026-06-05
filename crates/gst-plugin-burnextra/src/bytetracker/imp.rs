use gst::glib::{self, ParamSpecBuilderExt};
use gst::prelude::GstParamSpecBuilderExt;
use gst::subclass::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use gst_analytics::ffi::gst_analytics_relation_meta_add_tracking_mtd;
use gst_base::subclass::prelude::BaseTransformImpl;
use std::sync::{LazyLock, Mutex};

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "bytetracker",
        gst::DebugColorFlags::empty(),
        Some("ByteTracker Object Tracking Element"),
    )
});

// TODO: Update settings.
struct Settings {
    high_threshold: f32,
    low_threshold: f32,
    match_threshold: f32,
    max_time_lost: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            high_threshold: 0.6,
            low_threshold: 0.1,
            match_threshold: 0.2,
            max_time_lost: 30,
        }
    }
}

impl<'a> From<&'a Settings> for bytetrack::Settings {
    fn from(settings: &'a Settings) -> Self {
        Self {
            track_threshold: settings.high_threshold,
            low_threshold: settings.low_threshold,
            det_threshold: 0.1, // TODO
            match_threshold: settings.match_threshold,
            max_time_lost: settings.max_time_lost as usize,
        }
    }
}

#[derive(Default)]
pub struct ByteTracker {
    settings: Mutex<Settings>,
    bytetrack: Mutex<Option<bytetrack::ByteTrack>>,
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
                glib::ParamSpecFloat::builder("high-threshold")
                    .nick("High Threshold")
                    .blurb("TODO")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().high_threshold)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecFloat::builder("low-threshold")
                    .nick("Low Threshold")
                    .blurb("TODO")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().low_threshold)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecFloat::builder("match-threshold")
                    .nick("Match Threshold")
                    .blurb("TODO")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().match_threshold)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("max-lost")
                    .nick("Maximum Lost")
                    .blurb("TODO")
                    .default_value(Settings::default().max_time_lost)
                    .mutable_ready()
                    .build(),
            ]
        });

        &*PROPERTIES
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

        println!("Getting metadata");

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

        println!("Getting pad templates");

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for ByteTracker {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let settings = bytetrack::Settings::from(&*self.settings.lock().unwrap());
        let mut bytetrack = self.bytetrack.lock().unwrap();
        *bytetrack = Some(bytetrack::ByteTrack::new(settings));

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.bytetrack.lock().unwrap() = None;

        gst::info!(CAT, imp = self, "Stopped");

        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut bytetrack_guard = self.bytetrack.lock().unwrap();
        let Some(bytetrack) = &mut *bytetrack_guard else {
            gst::error!(CAT, imp = self, "Wrong state");
            return Err(gst::FlowError::Flushing);
        };

        let detections = detections(buffer);
        for track in bytetrack.update(&detections) {
            set_track_id(buffer, &track, buffer.pts().unwrap());
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

fn set_track_id(
    buffer: &mut gst::BufferRef,
    track: &bytetrack::STrack,
    first_seen: gst::ClockTime,
) {
    let Some(idx) = track.det_idx() else {
        gst::debug!(CAT, "No track index for strack");
        return;
    };

    // TODO: Replace this with generics over Detection in bytetrack
    // to keep the meta index and the od index inside the meta, instead
    // of just the absolute index in the strack.
    let mut total_idx = 0;
    let mut meta_idx = 0;
    let mut od_id = None;
    'meta: for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
        for od in meta.iter::<gst_analytics::AnalyticsODMtd>() {
            if od.location().is_err() {
                gst::warning!(CAT, "Failed to get location from object detection metadata");
                continue;
            }

            if total_idx == idx {
                od_id = Some(od.id());
                break 'meta;
            } else {
                total_idx += 1;
            }
        }

        if total_idx >= idx {
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
            track.track_id() as u64,
            first_seen.nseconds(),
            mtd.as_mut_ptr(),
        );
        let mtd = mtd.assume_init();
        mtd.id
    };
    meta.set_relation(gst_analytics::RelTypes::RELATE_TO, od_id, tracking_id)
        .unwrap()
}

fn detections(buffer: &gst::BufferRef) -> Vec<bytetrack::Detection> {
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

fn location_to_detection(location: &gst_analytics::AnalyticsODLocation) -> bytetrack::Detection {
    let score = location.loc_conf_lvl;
    let x = location.x as f32;
    let y = location.y as f32;
    let w = location.w as f32;
    let h = location.h as f32;
    bytetrack::Detection {
        score,
        bbox: bytetrack::BoundingBox::from_tlwh(x, y, w, h),
    }
}

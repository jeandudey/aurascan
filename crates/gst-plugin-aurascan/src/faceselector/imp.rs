use std::sync::{LazyLock, Mutex};

use gst::glib;
use gst::glib::ParamSpecBuilderExt;
use gst::glib::value::ToValue;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use gst_base::subclass::prelude::BaseTransformImpl;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "faceselector",
        gst::DebugColorFlags::empty(),
        Some("Detection Crop Metadata Element"),
    )
});

#[derive(Default)]
pub struct FaceSelector {
    settings: Mutex<Settings>,
    selector: Mutex<Option<aurascan_faceselector::FaceSelector>>,
    cropper: Mutex<Option<aurascan_smoothedcrop::SmoothedCrop>>,
    video_info: Mutex<Option<gst_video::VideoInfo>>,
}

#[glib::object_subclass]
impl ObjectSubclass for FaceSelector {
    const NAME: &'static str = "GstAscDetectionCropMeta";

    type Type = super::FaceSelector;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for FaceSelector {
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

impl GstObjectImpl for FaceSelector {}

impl ElementImpl for FaceSelector {
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

impl BaseTransformImpl for FaceSelector {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        *self.selector.lock().unwrap() = Some(aurascan_faceselector::FaceSelector::new());
        *self.cropper.lock().unwrap() = Some(aurascan_smoothedcrop::SmoothedCrop::new());

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.selector.lock().unwrap() = None;
        *self.cropper.lock().unwrap() = None;
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
        let mut selector_guard = self.selector.lock().unwrap();
        let Some(selector) = &mut *selector_guard else {
            gst::error!(CAT, imp = self, "Wrong state, selector not started");
            return Err(gst::FlowError::Flushing);
        };

        let mut cropper_guard = self.cropper.lock().unwrap();
        let Some(cropper) = &mut *cropper_guard else {
            gst::error!(CAT, imp = self, "Wrong state, cropper not started");
            return Err(gst::FlowError::Flushing);
        };

        let (frame_w, frame_h) = self
            .video_info
            .lock()
            .map(|video_info| {
                (
                    video_info.as_ref().unwrap().width(),
                    video_info.as_ref().unwrap().height(),
                )
            })
            .unwrap();

        let settings = self.settings.lock().unwrap();

        let detections = detections(buffer);

        let chosen_id = selector.select(
            detections.iter().cloned(),
            frame_w as f32,
            frame_h as f32,
            settings.score_margin,
            settings.missing_threshold,
        );

        // When a detection is selected, advance the EMA toward its location.
        let location = chosen_id.and_then(|chosen_id| find_location(chosen_id, buffer));
        if let Some(location) = location.as_ref() {
            cropper.advance(
                aurascan_smoothedcrop::Rect {
                    x: location.x as u32,
                    y: location.y as u32,
                    width: location.w as u32,
                    height: location.h as u32,
                },
                &aurascan_smoothedcrop::Settings {
                    expansion_top: settings.expansion_top,
                    expansion_bottom: settings.expansion_bottom,
                    expansion_side: settings.expansion_side,
                    alpha: settings.smoothing,
                },
            );
        }

        // Always emit a rect: last known smoothed position, or full frame when
        // no detection has ever been seen.
        let crop = cropper.rect(frame_w, frame_h);

        // Add the crop metadata for the selected face and emit custom
        // metadata about what face was selected.
        gst_video::VideoCropMeta::add(buffer, (crop.x, crop.y, crop.width, crop.height));
        add_selected_face_meta(buffer, location, chosen_id)?;

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

fn add_selected_face_meta(
    buffer: &mut gst::BufferRef,
    location: Option<gst_analytics::AnalyticsODLocation>,
    id: Option<u64>,
) -> Result<(), gst::FlowError> {
    let Some(location) = location else {
        return Ok(());
    };

    let mut meta = match gst::meta::CustomMeta::add(buffer, "SelectedFaceMeta") {
        Ok(meta) => meta,
        Err(err) => {
            gst::error!(CAT, "Failed to add SelectedFaceMeta: {err}");
            return Err(gst::FlowError::Flushing);
        }
    };

    let structure = meta.mut_structure();
    structure.set("x", location.x);
    structure.set("y", location.y);
    structure.set("w", location.w);
    structure.set("h", location.h);
    structure.set("loc-conf-lvl", location.loc_conf_lvl);

    if let Some(id) = id {
        structure.set("id", id);
    }

    Ok(())
}

struct Settings {
    expansion_top: f32,
    expansion_bottom: f32,
    expansion_side: f32,
    smoothing: f32,
    score_margin: f32,
    missing_threshold: usize,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            expansion_top: 0.45,
            expansion_bottom: 0.15,
            expansion_side: 0.25,
            smoothing: 0.3,
            score_margin: 0.15,
            missing_threshold: 15,
        }
    }
}

#[derive(Debug, Clone)]
struct Detection {
    location: gst_analytics::AnalyticsODLocation,
    id: u64,
}

impl aurascan_faceselector::FaceDetection for Detection {
    fn x(&self) -> f32 {
        self.location.x as f32
    }

    fn y(&self) -> f32 {
        self.location.y as f32
    }

    fn w(&self) -> f32 {
        self.location.w as f32
    }

    fn h(&self) -> f32 {
        self.location.h as f32
    }

    fn score(&self) -> f32 {
        self.location.loc_conf_lvl
    }

    fn id(&self) -> u64 {
        self.id
    }
}

fn detections(buffer: &gst::BufferRef) -> Vec<Detection> {
    let Some(meta) = buffer.meta::<gst_analytics::AnalyticsRelationMeta>() else {
        return Vec::new();
    };

    meta.iter::<gst_analytics::AnalyticsODMtd>()
        .filter_map(|odmeta| {
            let id = object_detection_tracking_id(&meta, &odmeta)?;
            Some(Detection {
                id,
                location: odmeta
                    .location()
                    .inspect_err(|err| {
                        gst::error!(CAT, "Failed to retrieve AnalyticsODMtd location: {err}");
                    })
                    .ok()?,
            })
        })
        .collect()
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

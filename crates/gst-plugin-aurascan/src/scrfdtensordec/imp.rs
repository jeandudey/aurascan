use std::sync::{LazyLock, Mutex};

use byte_slice_cast::AsSliceOf;
use gst::glib;
use gst::subclass::prelude::*;
use gst_analytics::prelude::*;
use gst_base::subclass::prelude::BaseTransformImpl;
use gst_video::prelude::*;

use crate::scrfd;
use crate::scrfd::{
    ModelKind, SCRFD_BBOX8_OUT_ID, SCRFD_BBOX16_OUT_ID, SCRFD_BBOX32_OUT_ID, SCRFD_GROUP_ID,
    SCRFD_KPS_GROUP_ID, SCRFD_KPS8_OUT_ID, SCRFD_KPS16_OUT_ID, SCRFD_KPS32_OUT_ID,
    SCRFD_SCORE8_OUT_ID, SCRFD_SCORE16_OUT_ID, SCRFD_SCORE32_OUT_ID,
};

const NUM_ANCHORS_PER_CELL: usize = 2;
const FACE_CLASS_LABEL: &glib::GStr = glib::gstr!("face");

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "scrfdtensordec",
        gst::DebugColorFlags::empty(),
        Some("SCRFD Tensor Decoder"),
    )
});

struct Detection {
    score: f32,
    bbox: [f32; 4],
    kps: Option<[[f32; 2]; 5]>,
}

impl Detection {
    fn to_oriented_od_params(
        &self,
        video_size: Option<(i32, i32)>,
    ) -> Option<(i32, i32, i32, i32)> {
        let [min_x, min_y, max_x, max_y] = self.bbox;
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return None;
        }

        let (x0, y0, x1, y1) = (min_x.floor(), min_y.floor(), max_x.ceil(), max_y.ceil());
        if x1 <= x0 || y1 <= y0 {
            return None;
        }

        if let Some((frame_width, frame_height)) = video_size
            && frame_width > 0
            && frame_height > 0
        {
            let fw = frame_width as f32;
            let fh = frame_height as f32;

            if x1 <= 0.0 || y1 <= 0.0 || x0 >= fw || y0 >= fh {
                return None;
            }
        }

        let x = x0 as i32;
        let y = y0 as i32;
        let w = (x1 - x0) as i32;
        let h = (y1 - y0) as i32;

        if w <= 0 || h <= 0 {
            return None;
        }

        Some((x, y, w, h))
    }
}

struct Settings {
    score_threshold: f32,
    iou_threshold: f32,
    max_detections: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            score_threshold: 0.5,
            iou_threshold: 0.5,
            max_detections: 100,
        }
    }
}

#[derive(Default)]
pub struct ScrfdTensorDec {
    settings: Mutex<Settings>,
    state: Mutex<Option<State>>,
}

struct State {
    video_info: gst_video::VideoInfo,
    model_kind: scrfd::ModelKind,
}

#[glib::object_subclass]
impl ObjectSubclass for ScrfdTensorDec {
    const NAME: &'static str = "GstAscScrfdtensorDec";
    type Type = super::ScrfdTensorDec;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for ScrfdTensorDec {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecFloat::builder("score-threshold")
                    .nick("Detection Score Threshold")
                    .blurb("The detections with score lower to this threshold will be excluded")
                    .minimum(0.0)
                    .maximum(1.0)
                    .default_value(Settings::default().score_threshold)
                    .mutable_playing()
                    .build(),
                glib::ParamSpecFloat::builder("iou-threshold")
                    .nick("IoU Threshold")
                    .blurb("Maximum intersection-over-union between bounding boxes to consider them distinct")
                    .default_value(Settings::default().iou_threshold)
                    .mutable_playing()
                    .build(),
                glib::ParamSpecUInt::builder("max-detections")
                    .nick("Maximum Detections")
                    .blurb("The maximum number of detections")
                    .default_value(Settings::default().max_detections)
                    .mutable_ready()
                    .build(),
            ]
        });

        &PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "score-threshold" => {
                let mut settings = self.settings.lock().unwrap();
                settings.score_threshold = value.get().unwrap();
            }
            "iou-threshold" => {
                let mut settings = self.settings.lock().unwrap();
                settings.iou_threshold = value.get().unwrap();
            }
            "max-detections" => {
                let mut settings = self.settings.lock().unwrap();
                settings.max_detections = value.get().unwrap();
            }
            _ => {}
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "score-threshold" => {
                let settings = self.settings.lock().unwrap();
                settings.score_threshold.to_value()
            }
            "iou-threshold" => {
                let settings = self.settings.lock().unwrap();
                settings.iou_threshold.to_value()
            }
            "max-detections" => {
                let settings = self.settings.lock().unwrap();
                settings.max_detections.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for ScrfdTensorDec {}

impl ElementImpl for ScrfdTensorDec {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "SCRFD Tensor Decoder Element",
                "Tensordecoder/Video",
                "Decodes tensors from SCRFD model",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let mut sink_caps = gst::Caps::new_empty();
            sink_caps.merge(tensorgroups(true));
            sink_caps.merge(tensorgroups(false));

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &sink_caps,
            )
            .unwrap();

            let src_caps = gst_video::VideoCapsBuilder::new().build();
            let src_pad_template = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &src_caps,
            )
            .unwrap();

            vec![sink_pad_template, src_pad_template]
        });

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for ScrfdTensorDec {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        gst::info!(CAT, imp = self, "Started");
        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.state.lock().unwrap() = None;
        gst::info!(CAT, imp = self, "Stopped");
        Ok(())
    }

    fn set_caps(&self, incaps: &gst::Caps, _outcaps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let video_info = gst_video::VideoInfo::from_caps(incaps)
            .map_err(|_| gst::loggable_error!(CAT, "Invalid caps {incaps:?}"))?;
        let model_kind = scrfd::ModelKind::from_caps(incaps).ok_or_else(|| {
            gst::loggable_error!(CAT, "Could not determine model kind from caps {incaps:?}")
        })?;

        *self.state.lock().unwrap() = Some(State {
            video_info,
            model_kind,
        });

        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let settings = self.settings.lock().unwrap();

        let Some(state) = &*self.state.lock().unwrap() else {
            gst::error!(CAT, imp = self, "Invalid state");
            return Ok(gst::FlowSuccess::Ok);
        };

        let video_size = (
            state.video_info.width() as i32,
            state.video_info.height() as i32,
        );

        let mut detections = Vec::new();
        for strides in Stride::from_buffer(buffer, state.model_kind) {
            for stride in strides {
                stride.decode_into(
                    &mut detections,
                    settings.score_threshold,
                    state.video_info.width(),
                    state.video_info.height(),
                );
            }
        }

        if detections.is_empty() {
            return Ok(gst::FlowSuccess::Ok);
        }

        let mut detections = nms(detections, settings.iou_threshold);
        detections.truncate(settings.max_detections as usize);

        if detections.is_empty() {
            return Ok(gst::FlowSuccess::Ok);
        }

        add_detection_meta(buffer, &detections, video_size);

        Ok(gst::FlowSuccess::Ok)
    }
}

#[cfg_attr(not(feature = "v1_30"), allow(unused_variables))]
fn add_detection_meta(
    buffer: &mut gst::BufferRef,
    detections: &[Detection],
    video_size: (i32, i32),
) {
    let face_class_label = glib::Quark::from_static_str(FACE_CLASS_LABEL);

    let mut meta = gst_analytics::AnalyticsRelationMeta::add(buffer);
    for detection in detections {
        let Some((x, y, w, h)) = detection.to_oriented_od_params(Some(video_size)) else {
            gst::debug!(CAT, "Skipping invalid/out-of-frame hand bbox");
            continue;
        };

        let od_meta = match meta.add_od_mtd(face_class_label, x, y, w, h, detection.score) {
            Ok(v) => v,
            Err(err) => {
                gst::warning!(CAT, "Failed to add oriented OD metadata: {err}");
                continue;
            }
        };

        if let Some(points) = detection.kps {
            #[cfg(feature = "v1_30")]
            {
                for point in points {
                    match meta.add_keypoint_mtd(
                        gst_analytics::AnalyticsKeypointDimensions::_2d,
                        point[0] as i32,
                        point[1] as i32,
                        0,
                        gst_analytics::AnalyticsKeypointVisibility::VISIBLE,
                        detection.score,
                    ) {
                        Ok(keypoint_meta) => {
                            if let Err(err) = meta.set_relation(
                                gst_analytics::RelTypes::RELATE_TO,
                                od_meta.id(),
                                keypoint_meta.id(),
                            ) {
                                gst::warning!(
                                    CAT,
                                    "Failed to set relation between OD and keypoint: {err}"
                                );
                            }
                        }
                        Err(err) => {
                            gst::warning!(CAT, "Failed to add keypoint metadata: {err}");
                        }
                    }
                }
            }
        } else {
            gst::warning!(CAT, "can't add keypoint metadata, ignoring");
        }
    }
}

struct Stride {
    pub score: Vec<f32>,
    pub bbox: Vec<f32>,
    pub kps: Vec<f32>,
    pub num_anchors: usize,
    pub stride: usize,
}

impl Stride {
    pub fn from_tensors(
        score: &gst_analytics::Tensor,
        bbox: &gst_analytics::Tensor,
        kps: Option<&gst_analytics::Tensor>,
        stride: usize,
    ) -> Option<Self> {
        if score.dims()[1] != bbox.dims()[1] {
            gst::warning!(CAT, "The score and bbox num_anchor dimension doesn't match");
            return None;
        }

        if let Some(kps) = kps {
            if score.dims()[1] != kps.dims()[1] {
                gst::warning!(CAT, "The score and kps num_anchor dimension doesn't match");
                return None;
            }
        }

        Some(Self {
            score: extract_tensor_f32(score)?,
            bbox: extract_tensor_f32(bbox)?,
            kps: kps.and_then(extract_tensor_f32).unwrap_or_default(),
            num_anchors: score.dims()[1],
            stride,
        })
    }

    pub fn from_meta(
        meta: &gst::MetaRef<gst_analytics::TensorMeta>,
        model_kind: ModelKind,
    ) -> Option<[Self; 3]> {
        let score8 = typed_tensor(meta, SCRFD_SCORE8_OUT_ID, 1)?;
        let score16 = typed_tensor(meta, SCRFD_SCORE16_OUT_ID, 1)?;
        let score32 = typed_tensor(meta, SCRFD_SCORE32_OUT_ID, 1)?;

        let bbox8 = typed_tensor(meta, SCRFD_BBOX8_OUT_ID, 4)?;
        let bbox16 = typed_tensor(meta, SCRFD_BBOX16_OUT_ID, 4)?;
        let bbox32 = typed_tensor(meta, SCRFD_BBOX32_OUT_ID, 4)?;

        if model_kind == ModelKind::Kps {
            let kps8 = typed_tensor(meta, SCRFD_KPS8_OUT_ID, 10)?;
            let kps16 = typed_tensor(meta, SCRFD_KPS16_OUT_ID, 10)?;
            let kps32 = typed_tensor(meta, SCRFD_KPS32_OUT_ID, 10)?;
            Some([
                Stride::from_tensors(score8, bbox8, Some(kps8), 8)?,
                Stride::from_tensors(score16, bbox16, Some(kps16), 16)?,
                Stride::from_tensors(score32, bbox32, Some(kps32), 32)?,
            ])
        } else {
            Some([
                Stride::from_tensors(score8, bbox8, None, 8)?,
                Stride::from_tensors(score16, bbox16, None, 16)?,
                Stride::from_tensors(score32, bbox32, None, 32)?,
            ])
        }
    }

    pub fn from_buffer<'a>(
        buffer: &'a gst::BufferRef,
        model_kind: ModelKind,
    ) -> impl Iterator<Item = [Self; 3]> + 'a {
        buffer
            .iter_meta::<gst_analytics::TensorMeta>()
            .filter_map(move |meta| Self::from_meta(&meta, model_kind))
    }

    pub fn decode_into(
        &self,
        detections: &mut Vec<Detection>,
        threshold: f32,
        width: u32,
        height: u32,
    ) {
        let feat_w = (width as usize).div_ceil(self.stride);
        let feat_h = (height as usize).div_ceil(self.stride);

        if feat_w * feat_h * NUM_ANCHORS_PER_CELL != self.num_anchors {
            gst::error!(
                CAT,
                "width and height don't match tensor size: feat_w={feat_w}, feat_h={feat_h}, num_anchors={}",
                self.num_anchors
            );
            return;
        }

        let mut i = 0;
        for y in 0..feat_h {
            for x in 0..feat_w {
                for _ in 0..NUM_ANCHORS_PER_CELL {
                    let score = self.score[i];
                    if score >= threshold {
                        let cx = (x * self.stride) as f32;
                        let cy = (y * self.stride) as f32;

                        let dist = &self.bbox[i * 4..i * 4 + 4];
                        let bbox = [
                            cx - dist[0] * self.stride as f32,
                            cy - dist[1] * self.stride as f32,
                            cx + dist[2] * self.stride as f32,
                            cy + dist[3] * self.stride as f32,
                        ];

                        let kps = if !self.kps.is_empty() {
                            let kps = &self.kps[i * 10..i * 10 + 10];
                            let mut points = [[0.0; 2]; 5];
                            for i in 0..5 {
                                points[i][0] = cx + kps[i * 2] * self.stride as f32;
                                points[i][1] = cy + kps[i * 2 + 1] * self.stride as f32;
                            }
                            Some(points)
                        } else {
                            None
                        };

                        detections.push(Detection { score, bbox, kps });
                    }
                    i += 1;
                }
            }
        }
    }
}

fn typed_tensor<'a>(
    meta: &'a gst::MetaRef<gst_analytics::TensorMeta>,
    id: &'static glib::GStr,
    channels: usize,
) -> Option<&'a gst_analytics::Tensor> {
    meta.typed_tensor(
        glib::Quark::from_static_str(id),
        gst_analytics::TensorDataType::Float32,
        gst_analytics::TensorDimOrder::RowMajor,
        &[1, usize::MAX, channels],
    )
}

fn extract_tensor_f32(tensor: &gst_analytics::Tensor) -> Option<Vec<f32>> {
    Some(
        tensor
            .data()
            .map_readable()
            .ok()?
            .as_slice_of::<f32>()
            .ok()?
            .to_vec(),
    )
}

fn iou(a: &[f32; 4], b: &[f32; 4]) -> f32 {
    let x1 = a[0].max(b[0]);
    let y1 = a[1].max(b[1]);
    let x2 = a[2].min(b[2]);
    let y2 = a[3].min(b[3]);

    let w = (x2 - x1).max(0.0);
    let h = (y2 - y1).max(0.0);
    let inter = w * h;

    let area_a = (a[2] - a[0]).max(0.0) * (a[3] - a[1]).max(0.0);
    let area_b = (b[2] - b[0]).max(0.0) * (b[3] - b[1]).max(0.0);
    let union = area_a + area_b - inter;

    if union <= 0.0 { 0.0 } else { inter / union }
}

fn nms(mut dets: Vec<Detection>, iou_threshold: f32) -> Vec<Detection> {
    // sort by score, highest first
    dets.sort_unstable_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

    let mut keep: Vec<Detection> = Vec::new();
    'outer: for det in dets {
        for kept in &keep {
            if iou(&det.bbox, &kept.bbox) > iou_threshold {
                continue 'outer;
            }
        }
        keep.push(det);
    }
    keep
}

fn strided(tensor_id: &'static glib::GStr, channels: i32) -> gst::Caps {
    gst::Caps::builder("tensor/strided")
        .field("tensor-id", tensor_id)
        .field(
            "dims",
            gst::Array::from_values([
                1i32.to_send_value(),
                0i32.to_send_value(),
                channels.to_send_value(),
            ]),
        )
        .build()
}

fn tensorgroups(is_kps: bool) -> gst::Caps {
    let v_tensor_s = if is_kps {
        gst::UniqueList::new([
            strided(SCRFD_SCORE8_OUT_ID, 1),
            strided(SCRFD_SCORE16_OUT_ID, 1),
            strided(SCRFD_SCORE32_OUT_ID, 1),
            strided(SCRFD_BBOX8_OUT_ID, 4),
            strided(SCRFD_BBOX16_OUT_ID, 4),
            strided(SCRFD_BBOX32_OUT_ID, 4),
            strided(SCRFD_KPS8_OUT_ID, 10),
            strided(SCRFD_KPS16_OUT_ID, 10),
            strided(SCRFD_KPS32_OUT_ID, 10),
        ])
    } else {
        gst::UniqueList::new([
            strided(SCRFD_SCORE8_OUT_ID, 1),
            strided(SCRFD_SCORE16_OUT_ID, 1),
            strided(SCRFD_SCORE32_OUT_ID, 1),
            strided(SCRFD_BBOX8_OUT_ID, 4),
            strided(SCRFD_BBOX16_OUT_ID, 4),
            strided(SCRFD_BBOX32_OUT_ID, 4),
        ])
    };
    let group_id = if is_kps {
        SCRFD_KPS_GROUP_ID
    } else {
        SCRFD_GROUP_ID
    };
    let tensors = gst::Structure::builder("tensorgroups")
        .field(group_id, v_tensor_s)
        .build();

    gst_video::VideoCapsBuilder::new()
        .field("tensors", tensors)
        .build()
}

use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

use byte_slice_cast::AsSliceOf;
use gst::glib;
use gst::subclass::prelude::*;
use gst_analytics::prelude::*;
use gst_base::subclass::prelude::BaseTransformImpl;
use gst_video::prelude::*;

const NUM_ANCHORS: usize = 2;

const GROUP_ID: &glib::GStr = glib::gstr!("scrfd");
const GROUP_ID_KPS: &glib::GStr = glib::gstr!("scrfd-kps");
const SCRFD_SCORE: &glib::GStr = glib::gstr!("scrfd-score-out");
const SCRFD_BBOX: &glib::GStr = glib::gstr!("scrfd-bbox-out");
const SCRFD_KPS: &glib::GStr = glib::gstr!("scrfd-kps-out");
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
    kp: Option<[[f32; 2]; 5]>,
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
    video_info: Mutex<Option<gst_video::VideoInfo>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ScrfdTensorDec {
    const NAME: &'static str = "GstBurnExtraScrfdtensorDec";
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
                    .nick("IOU Threshold")
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
        gst::info!(CAT, imp = self, "Stopped");
        Ok(())
    }

    fn set_caps(&self, incaps: &gst::Caps, _outcaps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let video_info = gst_video::VideoInfo::from_caps(incaps)
            .map_err(|_| gst::loggable_error!(CAT, "Invalid caps {incaps:?}"))?;
        *self.video_info.lock().unwrap() = Some(video_info);
        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let settings = self.settings.lock().unwrap();

        let video_size = self
            .video_info
            .lock()
            .unwrap()
            .as_ref()
            .map(|info| (info.width() as i32, info.height() as i32));

        let mut detections = Vec::new();
        for meta in buffer.iter_meta::<gst_analytics::TensorMeta>() {
            gst::trace!(CAT, imp = self, "Num tensors: {}", meta.as_slice().len());

            let score_tensors = find_tensors(&meta, SCRFD_SCORE, 1);
            let bbox_tensors = find_tensors(&meta, SCRFD_BBOX, 4);
            let kps_tensors = find_tensors(&meta, SCRFD_KPS, 10);

            gst::trace!(CAT, imp = self, "Num score: {}", score_tensors.len());
            gst::trace!(CAT, imp = self, "Num bbox: {}", bbox_tensors.len());
            gst::trace!(CAT, imp = self, "Num kps: {}", kps_tensors.len());

            let tensors = group_tensors(&score_tensors, &bbox_tensors, &kps_tensors);
            gst::trace!(CAT, imp = self, "Num grouped tensors: {}", tensors.len());

            let tensors = convert_tensors(tensors);

            let feat_max = tensors
                .iter()
                .map(|((_, dims), _, _)| ((dims[1] / NUM_ANCHORS) as f64).sqrt() as usize)
                .fold(0, |a, b| a.max(b));

            for (scores, bboxes, kpses) in tensors {
                let (scores, scores_dims) = scores;
                let (bboxes, _) = bboxes;

                let feat = ((scores_dims[1] / NUM_ANCHORS) as f64).sqrt() as usize;
                let stride = 8 * feat_max / feat; // 8, 16, or 32

                gst::trace!(CAT, imp = self, "Stride: {}", stride);
                let n = scores_dims[1];
                for i in 0..n {
                    let s = scores[i];
                    if s < settings.score_threshold {
                        continue;
                    }

                    let point = i / NUM_ANCHORS;
                    let gx = point % feat;
                    let gy = point / feat;

                    let cx = (gx * stride) as f32;
                    let cy = (gy * stride) as f32;

                    let b = i * 4;
                    let l = bboxes[b] * stride as f32;
                    let t = bboxes[b + 1] * stride as f32;
                    let r = bboxes[b + 2] * stride as f32;
                    let d = bboxes[b + 3] * stride as f32;

                    let bbox = [cx - l, cy - t, cx + r, cy + d];

                    let kp = kpses.as_ref().map(|(kps, _)| {
                        let mut pts = [[0.0f32; 2]; 5];
                        let base = i * 10;
                        for k in 0..5 {
                            pts[k][0] = cx + kps[base + k * 2] * stride as f32;
                            pts[k][1] = cy + kps[base + k * 2 + 1] * stride as f32;
                        }
                        pts
                    });

                    detections.push(Detection { score: s, bbox, kp });
                }
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

        gst::trace!(CAT, imp = self, "Num detections: {}", detections.len());

        let mut rmeta = gst_analytics::AnalyticsRelationMeta::add(buffer);
        let class = glib::Quark::from_static_str(FACE_CLASS_LABEL);

        let mut count = 0;
        for detection in &detections {
            gst::debug!(
                CAT,
                imp = self,
                "Face: bbox={:?}, score={}, kp={:?}",
                detection.bbox,
                detection.score,
                detection.kp
            );

            let Some((x, y, w, h)) = detection.to_oriented_od_params(video_size) else {
                gst::debug!(CAT, imp = self, "Skipping invalid/out-of-frame hand bbox");
                continue;
            };

            if let Err(err) = rmeta.add_od_mtd(class, x, y, w, h, detection.score) {
                gst::warning!(CAT, "Failed to add oriented OD metadata: {err}");
            }

            count += 1;
        }
        gst::debug!(CAT, imp = self, "Added {count} faces to OD metadata");

        Ok(gst::FlowSuccess::Ok)
    }
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

fn convert_tensors<'a>(
    tensors: Vec<(
        &'a gst_analytics::Tensor,
        &'a gst_analytics::Tensor,
        Option<&'a gst_analytics::Tensor>,
    )>,
) -> Vec<(
    (Vec<f32>, [usize; 3]),
    (Vec<f32>, [usize; 3]),
    Option<(Vec<f32>, [usize; 3])>,
)> {
    tensors
        .into_iter()
        .filter_map(|(score, bbox, kps)| {
            let score_map = score.data().map_readable().ok()?;
            let bbox_map = bbox.data().map_readable().ok()?;
            let kps_map = kps.and_then(|kps| kps.data().map_readable().ok());

            let score_data = score_map.as_slice_of::<f32>().ok()?.to_vec();
            let bbox_data = bbox_map.as_slice_of::<f32>().ok()?.to_vec();
            let kps_data = kps_map.and_then(|kps| Some(kps.as_slice_of::<f32>().ok()?.to_vec()));

            let score_dims = score.dims().try_into().unwrap();
            let bbox_dims = bbox.dims().try_into().unwrap();
            let kps_dims = kps.map(|t| t.dims().try_into().unwrap());
            Some((
                (score_data, score_dims),
                (bbox_data, bbox_dims),
                kps_data.and_then(|kps_data| Some((kps_data, kps_dims?))),
            ))
        })
        .collect()
}

fn group_tensors<'a>(
    scores: &HashMap<usize, &'a gst_analytics::Tensor>,
    bboxes: &HashMap<usize, &'a gst_analytics::Tensor>,
    kps: &HashMap<usize, &'a gst_analytics::Tensor>,
) -> Vec<(
    &'a gst_analytics::Tensor,
    &'a gst_analytics::Tensor,
    Option<&'a gst_analytics::Tensor>,
)> {
    scores
        .into_iter()
        .filter_map(|(anchors, &score)| {
            Some((score, *bboxes.get(&anchors)?, kps.get(&anchors).map(|v| *v)))
        })
        .collect()
}

fn find_tensors<'a>(
    meta: &'a gst::MetaRef<'a, gst_analytics::TensorMeta>,
    id: &'static glib::GStr,
    channels: usize,
) -> HashMap<usize, &'a gst_analytics::Tensor> {
    meta.as_slice()
        .into_iter()
        .filter(|tensor| {
            let is_score = tensor.id() == glib::Quark::from_static_str(id);
            let is_correct_data_type = tensor.check_type(
                gst_analytics::TensorDataType::Float32,
                gst_analytics::TensorDimOrder::RowMajor,
                &[1, usize::MAX, channels],
            );
            is_score && is_correct_data_type
        })
        .map(|tensor| (tensor.dims()[1], tensor))
        .collect()
}

fn strided(field_id: &str, channels: i32) -> gst::Caps {
    gst::Caps::builder("tensor/strided")
        .field("field-id", field_id)
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
            strided("scrfd-score", 1),
            strided("scrfd-score", 1),
            strided("scrfd-score", 1),
            strided("scrfd-bbox", 4),
            strided("scrfd-bbox", 4),
            strided("scrfd-bbox", 4),
            strided("scrfd-kps", 10),
            strided("scrfd-kps", 10),
            strided("scrfd-kps", 10),
        ])
    } else {
        gst::UniqueList::new([
            strided("scrfd-score", 1),
            strided("scrfd-score", 1),
            strided("scrfd-score", 1),
            strided("scrfd-bbox", 4),
            strided("scrfd-bbox", 4),
            strided("scrfd-bbox", 4),
        ])
    };
    let group_id = if is_kps { GROUP_ID_KPS } else { GROUP_ID };
    let tensors = gst::Structure::builder("tensorgroups")
        .field(group_id, v_tensor_s)
        .build();

    gst_video::VideoCapsBuilder::new()
        .field("tensors", tensors)
        .build()
}

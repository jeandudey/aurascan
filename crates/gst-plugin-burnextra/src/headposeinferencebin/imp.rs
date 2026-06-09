use gst::glib;
use gst::glib::object::Cast;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_analytics::AnalyticsMetaRefExt;
use std::sync::{Arc, LazyLock, Mutex, OnceLock};

#[derive(Default)]
pub struct HeadPoseInferenceBin {
    elements: OnceLock<Elements>,
    state: Arc<Mutex<Option<State>>>,
}

#[allow(dead_code)] // TODO: Remove.
#[derive(Debug)]
struct Elements {
    facedetectorinfernce: gst::Element,
    facedetectortensordec: gst::Element,
    tracker: gst::Element,
    videocrop: gst::Element,
    headposeinference: gst::Element,
    headposetensordec: gst::Element,
}

struct State {
    selector: Selector,
    cropper: Cropper,
}

struct Selector {
    /// Current locked ID, if any.
    locked_id: Option<u64>,
    /// The score of the new face must be higher than this to switch to it.
    switch_margin: f32,
    /// Frames to wait before dropping a lost ID.
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
    /// In `[x, y, w, h]`.
    bbox: [f32; 4],
    score: f32,
}

impl Detection {
    /// Compute the score for a given object detection.
    ///
    /// The score is computed as a weighted sum of the area, centrality, and confidence score.
    fn score(&self, frame_w: f32, frame_h: f32) -> f32 {
        let [x, y, w, h] = self.bbox;

        let area = (w * h) / (frame_w * frame_h); // normalized 0..1
        let cx = x + w * 0.5;
        let cy = y + h * 0.5;
        let dx = (cx - frame_w * 0.5) / frame_w;
        let dy = (cy - frame_h * 0.5) / frame_h;
        let centrality = 1.0 - (dx * dx + dy * dy).sqrt();

        0.6 * area + 0.3 * centrality + 0.1 * self.score
    }
}

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

    fn crop(
        &mut self,
        location: &gst_analytics::AnalyticsODLocation,
        frame_w: f32,
        frame_h: f32,
        alpha: f32, // EMA factor, e.g. 0.5 (higher = snappier, lower = smoother)
    ) -> (i32, i32, i32, i32) {
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

        // target geometry (center + square side) from this frame's detection
        let bw = right_raw - left_raw;
        let bh = bottom_raw - top_raw;
        let t_side = bw.max(bh);
        let t_cx = (left_raw + right_raw) / 2.0;
        let t_cy = (top_raw + bottom_raw) / 2.0;

        // EMA smoothing on center + side
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

        // cap smoothed side to the frame so the box always fits after shifting
        let side = self.side.min(frame_w - 1.0).min(frame_h - 1.0).max(1.0);
        let ccx = self.cx;
        let ccy = self.cy;

        let mut left = ccx - side / 2.0;
        let mut top = ccy - side / 2.0;
        let mut right = left + side;
        let mut bottom = top + side;

        // shift inward to preserve squareness near edges
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

        let crop_top = top.round() as i32;
        let crop_left = left.round() as i32;
        let crop_bottom = (frame_h - bottom).round() as i32;
        let crop_right = (frame_w - right).round() as i32;

        (crop_top, crop_left, crop_bottom, crop_right)
    }

    fn reset(&mut self) {
        self.initialized = false;
    }
}

#[glib::object_subclass]
impl ObjectSubclass for HeadPoseInferenceBin {
    const NAME: &'static str = "GstBurnExtraHeadPoseInferenceBin";

    type Type = super::HeadPoseInferenceBin;
    type ParentType = gst::Bin;
    type Interfaces = (gst::ChildProxy,);
}

impl ObjectImpl for HeadPoseInferenceBin {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        let bin = obj.upcast_ref::<gst::Bin>();

        *self.state.lock().unwrap() = Some(State {
            selector: Selector {
                locked_id: None,
                switch_margin: 0.15,
                grace_frames: 15,
                missing: 0,
            },
            cropper: Cropper::new(),
        });

        let facedetectorinfernce = gst::ElementFactory::make("burnextra-scrfdinference")
            .name("scrfdinference")
            .build()
            .unwrap();
        let facedetectortensordec = gst::ElementFactory::make("scrfdtensordec")
            .name("scrfdtensordec")
            .build()
            .unwrap();
        let tracker = gst::ElementFactory::make("bytetracker")
            .name("bytetracker")
            .build()
            .unwrap();
        let videocrop = gst::ElementFactory::make("videocrop")
            .name("videocrop")
            .build()
            .unwrap();
        let videoscale = gst::ElementFactory::make("videoscale")
            .name("videoscale")
            .build()
            .unwrap();
        let headposeinference = gst::ElementFactory::make("burnextra-sixdrepnet360inference")
            .name("sixdrepnet360inference")
            .build()
            .unwrap();
        let headposetensordec = gst::ElementFactory::make("sixdrepnet360tensordec")
            .name("sixdrepnet360tensordec")
            .build()
            .unwrap();

        let all = &[
            &facedetectorinfernce,
            &facedetectortensordec,
            &tracker,
            &videocrop,
            &videoscale,
            &headposeinference,
            &headposetensordec,
        ];
        bin.add_many(all).unwrap();
        gst::Element::link_many(all).unwrap();

        let sink_pad = facedetectorinfernce.static_pad("sink").unwrap();
        let ghost_sink = gst::GhostPad::with_target(&sink_pad).unwrap();
        ghost_sink.set_active(true).unwrap();
        bin.add_pad(&ghost_sink).unwrap();

        let src_pad = headposetensordec.static_pad("src").unwrap();
        let ghost_src = gst::GhostPad::with_target(&src_pad).unwrap();
        ghost_src.set_active(true).unwrap();
        bin.add_pad(&ghost_src).unwrap();

        let videocrop_sink = videocrop.static_pad("sink").unwrap();
        videocrop_sink.add_probe(
            gst::PadProbeType::BUFFER,
            glib::clone!(
                #[weak_allow_none]
                videocrop,
                #[weak_allow_none(rename_to = state)]
                self.state,
                move |pad, info| {
                    let Some(buffer) = info.buffer() else {
                        return gst::PadProbeReturn::Ok;
                    };

                    let Some(videocrop) = videocrop else {
                        return gst::PadProbeReturn::Ok;
                    };

                    let Some(video_info) = pad
                        .current_caps()
                        .and_then(|caps| gst_video::VideoInfo::from_caps(&caps).ok())
                    else {
                        return gst::PadProbeReturn::Ok;
                    };

                    let Some(state) = state else {
                        return gst::PadProbeReturn::Ok;
                    };

                    let mut state_guard = state.lock().unwrap();
                    let Some(state) = &mut *state_guard else {
                        return gst::PadProbeReturn::Ok;
                    };

                    let mut detections = Vec::new();
                    'meta: for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
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
                                continue 'meta;
                            };

                            let track_id = tr.info().0;

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
                    }

                    let Some(chosen_track_id) = state.selector.pick(
                        &detections,
                        video_info.width() as f32,
                        video_info.height() as f32,
                    ) else {
                        state.cropper.reset();
                        return gst::PadProbeReturn::Ok;
                    };

                    'meta: for meta in buffer.iter_meta::<gst_analytics::AnalyticsRelationMeta>() {
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

                            let track_id = tr.info().0;
                            if chosen_track_id == track_id {
                                let (top, left, bottom, right) = state.cropper.crop(
                                    &location,
                                    video_info.width() as f32,
                                    video_info.height() as f32,
                                    0.3,
                                );
                                videocrop.set_property("top", top);
                                videocrop.set_property("bottom", bottom);
                                videocrop.set_property("left", left);
                                videocrop.set_property("right", right);
                                break 'meta;
                            }
                        }
                    }

                    gst::PadProbeReturn::Ok
                }
            ),
        );

        self.elements
            .set(Elements {
                facedetectorinfernce,
                facedetectortensordec,
                tracker,
                videocrop,
                headposeinference,
                headposetensordec,
            })
            .unwrap();
    }
}

impl GstObjectImpl for HeadPoseInferenceBin {}

impl ElementImpl for HeadPoseInferenceBin {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Head Pose Inference Bin",
                "Filter/Video",
                "Face detection, tracking, and head pose estimation",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .build();
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

impl BinImpl for HeadPoseInferenceBin {}

impl ChildProxyImpl for HeadPoseInferenceBin {
    fn children_count(&self) -> u32 {
        self.obj().children().len() as u32
    }

    fn child_by_name(&self, name: &str) -> Option<glib::Object> {
        self.obj()
            .children()
            .iter()
            .find(|c| c.name() == name)
            .map(|c| c.clone().upcast())
    }

    fn child_by_index(&self, index: u32) -> Option<glib::Object> {
        self.obj()
            .children()
            .into_iter()
            .nth(index as usize)
            .map(|c| c.upcast())
    }
}

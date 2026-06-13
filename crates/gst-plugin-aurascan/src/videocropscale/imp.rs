use std::sync::{LazyLock, Mutex};

use gst::subclass::prelude::*;
use gst::{MetaAPI, glib};
use gst_base::subclass::prelude::*;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "videocropscale",
        gst::DebugColorFlags::empty(),
        Some("Video Crop and Scale"),
    )
});

struct State {
    in_info: gst_video::VideoInfo,
    out_info: gst_video::VideoInfo,
    /// Cached converter keyed by the last crop rect (x, y, w, h).
    converter: Option<(u32, u32, u32, u32, gst_video::VideoConverter)>,
}

#[derive(Default)]
pub struct VideoCropScale {
    state: Mutex<Option<State>>,
}

#[glib::object_subclass]
impl ObjectSubclass for VideoCropScale {
    const NAME: &'static str = "GstAuraScanVideoCropScale";
    type Type = super::VideoCropScale;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for VideoCropScale {}

impl GstObjectImpl for VideoCropScale {}

impl ElementImpl for VideoCropScale {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Video Crop Scale",
                "Filter/Effect/Video",
                "Crops and scales video using GstVideoCropMeta",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst_video::VideoCapsBuilder::new().build();
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

impl BaseTransformImpl for VideoCropScale {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::NeverInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = false;

    fn transform_caps(
        &self,
        _direction: gst::PadDirection,
        caps: &gst::Caps,
        filter: Option<&gst::Caps>,
    ) -> Option<gst::Caps> {
        // Allow any size on the other side; keep all other fields (format, framerate, etc.).
        let mut other_caps = caps.copy();
        for s in other_caps.make_mut().iter_mut() {
            s.set("width", gst::IntRange::new(1, i32::MAX));
            s.set("height", gst::IntRange::new(1, i32::MAX));
        }

        let other_caps = filter
            .map(|filter| other_caps.intersect_with_mode(filter, gst::CapsIntersectMode::First))
            .unwrap_or(other_caps);

        gst::debug!(
            CAT,
            imp = self,
            "transform_caps: other_caps={:?}, filter={:?}",
            other_caps,
            filter
        );

        Some(other_caps)
    }

    fn set_caps(&self, incaps: &gst::Caps, outcaps: &gst::Caps) -> Result<(), gst::LoggableError> {
        let in_info = gst_video::VideoInfo::from_caps(incaps)
            .map_err(|_| gst::loggable_error!(CAT, "Failed to parse input caps"))?;
        let out_info = gst_video::VideoInfo::from_caps(outcaps)
            .map_err(|_| gst::loggable_error!(CAT, "Failed to parse output caps"))?;

        gst::info!(
            CAT,
            imp = self,
            "{}x{} {:?} -> {}x{} {:?}",
            in_info.width(),
            in_info.height(),
            in_info.format(),
            out_info.width(),
            out_info.height(),
            out_info.format(),
        );

        *self.state.lock().unwrap() = Some(State {
            in_info,
            out_info,
            converter: None,
        });

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.state.lock().unwrap() = None;
        Ok(())
    }

    fn unit_size(&self, caps: &gst::Caps) -> Option<usize> {
        let info = gst_video::VideoInfo::from_caps(caps).ok()?;
        Some(info.size())
    }

    fn transform(
        &self,
        inbuf: &gst::Buffer,
        outbuf: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut guard = self.state.lock().unwrap();
        let Some(state) = guard.as_mut() else {
            gst::error!(CAT, imp = self, "Not configured");
            return Err(gst::FlowError::NotNegotiated);
        };

        let frame_w = state.in_info.width();
        let frame_h = state.in_info.height();

        // Read crop rect from buffer metadata; fall back to the full frame.
        let (cx, cy, cw, ch) = inbuf
            .meta::<gst_video::VideoCropMeta>()
            .map(|m| m.rect())
            .unwrap_or((0, 0, frame_w, frame_h));

        // Clamp to frame bounds so VideoConverter never receives out-of-range coords.
        let cx = cx.min(frame_w.saturating_sub(1));
        let cy = cy.min(frame_h.saturating_sub(1));
        let cw = cw.min(frame_w - cx).max(1);
        let ch = ch.min(frame_h - cy).max(1);

        gst::trace!(
            CAT,
            imp = self,
            "crop x={cx} y={cy} w={cw} h={ch} -> {}x{}",
            state.out_info.width(),
            state.out_info.height(),
        );

        // Create a new converter only when the crop rect changes.
        let needs_new = state.converter.as_ref().map_or(true, |&(x, y, w, h, _)| {
            x != cx || y != cy || w != cw || h != ch
        });

        if needs_new {
            let mut config = gst_video::VideoConverterConfig::new();
            config.set_src_x(cx as i32);
            config.set_src_y(cy as i32);
            config.set_src_width(Some(cw as i32));
            config.set_src_height(Some(ch as i32));
            config.set_dest_x(0);
            config.set_dest_y(0);
            config.set_dest_width(Some(state.out_info.width() as i32));
            config.set_dest_height(Some(state.out_info.height() as i32));

            match gst_video::VideoConverter::new(&state.in_info, &state.out_info, Some(config)) {
                Ok(conv) => state.converter = Some((cx, cy, cw, ch, conv)),
                Err(e) => {
                    gst::error!(CAT, imp = self, "Failed to create VideoConverter: {e}");
                    return Err(gst::FlowError::Error);
                }
            }
        }

        let conv = &state.converter.as_ref().unwrap().4;

        let in_frame = gst_video::VideoFrameRef::from_buffer_ref_readable(inbuf, &state.in_info)
            .map_err(|_| {
                gst::error!(CAT, imp = self, "Failed to map input frame");
                gst::FlowError::Error
            })?;

        let mut out_frame =
            gst_video::VideoFrameRef::from_buffer_ref_writable(outbuf, &state.out_info).map_err(
                |_| {
                    gst::error!(CAT, imp = self, "Failed to map output frame");
                    gst::FlowError::Error
                },
            )?;

        conv.frame_ref(&in_frame, &mut out_frame);

        Ok(gst::FlowSuccess::Ok)
    }

    fn transform_meta<'a>(
        &self,
        outbuf: &mut gst::BufferRef,
        meta: gst::MetaRef<'a, gst::Meta>,
        inbuf: &'a gst::BufferRef,
    ) -> bool {
        // Drop VideoCropMeta since we consume it.
        if meta.api() == gst_video::VideoCropMeta::meta_api() {
            return false;
        }

        self.parent_transform_meta(outbuf, meta, inbuf)
    }
}

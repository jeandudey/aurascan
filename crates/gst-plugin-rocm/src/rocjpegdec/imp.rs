// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::sync::{LazyLock, Mutex};

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_hip::prelude::*;
use gst_hip::{HipDevice, HipVideoFrameRef};
use gst_video::prelude::*;
use gst_video::subclass::prelude::*;
use gst_video::video_codec_state::Readable;
use gst_video::{VideoCapsBuilder, VideoCodecState, VideoFormat};
use rocjpeg::{ChromaSubsampling, DecodeParams, Decoder, ImageInfo, OutputFormat, Stream};

use super::Backend;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "rocjpegdec",
        gst::DebugColorFlags::empty(),
        Some("ROCm JPEG decoder"),
    )
});

const HIP_MEMORY_FEATURE: &str = "memory:HIPMemory";

const DEFAULT_DEVICE_ID: i32 = 0;

#[derive(Debug, Clone, Copy)]
struct Settings {
    device_id: i32,
    backend: Backend,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            device_id: DEFAULT_DEVICE_ID,
            backend: Backend::default(),
        }
    }
}

/// What the current output caps were negotiated for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct OutputConfig {
    width: u32,
    height: u32,
    subsampling: ChromaSubsampling,
    format: VideoFormat,
}

#[derive(Default)]
pub struct RocJpegDec {
    settings: Mutex<Settings>,
    device: Mutex<Option<HipDevice>>,
    decoder: Mutex<Option<Decoder>>,
    output: Mutex<Option<OutputConfig>>,
    input_state: Mutex<Option<VideoCodecState<'static, Readable>>>,
}

/// Output formats that can be produced for a given chroma subsampling, in order of preference,
/// with the rocJPEG output format used to produce them.
///
/// GRAY8 isn't offered because `GstHipAllocator` can't allocate it, grayscale images are
/// output as RGB instead.
fn candidate_formats(subsampling: ChromaSubsampling) -> &'static [(VideoFormat, OutputFormat)] {
    match subsampling {
        ChromaSubsampling::Yuv420 => &[
            (VideoFormat::Nv12, OutputFormat::Native),
            (VideoFormat::I420, OutputFormat::YuvPlanar),
            (VideoFormat::Rgb, OutputFormat::Rgb),
        ],
        ChromaSubsampling::Yuv422 => &[
            (VideoFormat::Y42b, OutputFormat::YuvPlanar),
            (VideoFormat::Rgb, OutputFormat::Rgb),
        ],
        ChromaSubsampling::Yuv444 => &[
            (VideoFormat::Y444, OutputFormat::YuvPlanar),
            (VideoFormat::Rgb, OutputFormat::Rgb),
        ],
        ChromaSubsampling::Yuv400 => &[(VideoFormat::Rgb, OutputFormat::Rgb)],
        _ => &[(VideoFormat::Rgb, OutputFormat::Rgb)],
    }
}

fn hip_caps(format: Option<VideoFormat>) -> gst::Caps {
    let mut builder = VideoCapsBuilder::new().features([HIP_MEMORY_FEATURE]);
    builder = match format {
        Some(format) => builder.format(format),
        None => builder.format_list([
            VideoFormat::Nv12,
            VideoFormat::I420,
            VideoFormat::Y42b,
            VideoFormat::Y444,
            VideoFormat::Rgb,
        ]),
    };
    builder.build()
}

impl RocJpegDec {
    fn device(&self) -> Option<HipDevice> {
        self.device.lock().unwrap().clone()
    }

    fn settings(&self) -> Settings {
        *self.settings.lock().unwrap()
    }

    /// Picks the output format the downstream elements accept and sets the output state.
    fn negotiate_output(
        &self,
        info: &ImageInfo,
    ) -> Result<(VideoFormat, OutputFormat), gst::FlowError> {
        let obj = self.obj();
        let allowed = obj.src_pad().allowed_caps();
        let candidates = candidate_formats(info.subsampling);

        let chosen = candidates.iter().find(|(format, _)| match &allowed {
            Some(allowed) => allowed.can_intersect(&hip_caps(Some(*format))),
            None => true,
        });
        let Some(&(format, output_format)) = chosen else {
            gst::error!(
                CAT,
                imp = self,
                "No supported output format accepted downstream"
            );
            return Err(gst::FlowError::NotNegotiated);
        };

        let input_state = self.input_state.lock().unwrap().clone();
        let mut state =
            obj.set_output_state(format, info.width(), info.height(), input_state.as_ref())?;
        if format != VideoFormat::Rgb {
            // JPEG data is full range BT.601 (sYCC).
            let colorimetry = gst_video::VideoColorimetry::new(
                gst_video::VideoColorRange::Range0_255,
                gst_video::VideoColorMatrix::Bt601,
                gst_video::VideoTransferFunction::Srgb,
                gst_video::VideoColorPrimaries::Bt709,
            );
            let (fps, par, interlace) = {
                let base = state.info();
                (base.fps(), base.par(), base.interlace_mode())
            };
            let info = gst_video::VideoInfo::builder(format, info.width(), info.height())
                .colorimetry(&colorimetry)
                .fps(fps)
                .par(par)
                .interlace_mode(interlace)
                .build()
                .map_err(|_| gst::FlowError::NotNegotiated)?;
            state.set_info(info);
        }
        let mut caps = state
            .info()
            .to_caps()
            .map_err(|_| gst::FlowError::NotNegotiated)?;
        caps.make_mut()
            .set_features_simple(Some(gst::CapsFeatures::new([HIP_MEMORY_FEATURE])));
        state.set_caps(&caps);
        obj.negotiate(state)?;

        gst::info!(
            CAT,
            imp = self,
            "Negotiated {:?} for {:?}",
            format,
            info.subsampling
        );
        Ok((format, output_format))
    }
}

#[glib::object_subclass]
impl ObjectSubclass for RocJpegDec {
    const NAME: &'static str = "GstRocJpegDec";
    type Type = super::RocJpegDec;
    type ParentType = gst_video::VideoDecoder;
}

impl ObjectImpl for RocJpegDec {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecInt::builder("device-id")
                    .nick("Device ID")
                    .blurb("HIP device used to decode, -1 selects any device")
                    .minimum(-1)
                    .default_value(DEFAULT_DEVICE_ID)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecEnum::builder_with_default("backend", Backend::default())
                    .nick("Backend")
                    .blurb("Hardware (VCN JPEG engine) or hybrid decoding")
                    .mutable_ready()
                    .build(),
            ]
        });
        PROPERTIES.as_ref()
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        let mut settings = self.settings.lock().unwrap();
        match pspec.name() {
            "device-id" => settings.device_id = value.get().expect("type checked upstream"),
            "backend" => settings.backend = value.get().expect("type checked upstream"),
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        let settings = self.settings.lock().unwrap();
        match pspec.name() {
            "device-id" => settings.device_id.to_value(),
            "backend" => settings.backend.to_value(),
            _ => unimplemented!(),
        }
    }

    fn constructed(&self) {
        self.parent_constructed();
        // One JPEG image per buffer.
        self.obj().set_packetized(true);
    }
}

impl GstObjectImpl for RocJpegDec {}

impl ElementImpl for RocJpegDec {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "ROCm JPEG decoder",
                "Codec/Decoder/Video/Hardware",
                "Decodes JPEG images to HIP memory using rocJPEG on AMD GPUs",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });
        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let sink_caps = gst::Caps::builder("image/jpeg")
                .field("width", gst::IntRange::new(1, 65535))
                .field("height", gst::IntRange::new(1, 65535))
                .build();
            let sink = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &sink_caps,
            )
            .unwrap();
            let src = gst::PadTemplate::new(
                "src",
                gst::PadDirection::Src,
                gst::PadPresence::Always,
                &hip_caps(None),
            )
            .unwrap();
            vec![sink, src]
        });
        PAD_TEMPLATES.as_ref()
    }

    fn set_context(&self, context: &gst::Context) {
        let settings = self.settings();
        if let Some(device) = gst_hip::hip_handle_set_context(
            &*self.obj(),
            context,
            gst_hip::HipVendor::Amd,
            settings.device_id,
        ) {
            *self.device.lock().unwrap() = Some(device);
        }
        self.parent_set_context(context);
    }
}

impl VideoDecoderImpl for RocJpegDec {
    fn open(&self) -> Result<(), gst::ErrorMessage> {
        let settings = self.settings();
        let device = gst_hip::hip_ensure_element_data(
            &*self.obj(),
            gst_hip::HipVendor::Amd,
            settings.device_id,
        );
        if device.is_none() {
            return Err(gst::error_msg!(
                gst::ResourceError::OpenRead,
                [
                    "Failed to get a HIP device (device-id={})",
                    settings.device_id
                ]
            ));
        }

        // Don't hold the lock: looking for a device queries and sets contexts on this element,
        // which lock it as well.
        *self.device.lock().unwrap() = device;

        Ok(())
    }

    fn close(&self) -> Result<(), gst::ErrorMessage> {
        *self.decoder.lock().unwrap() = None;
        *self.device.lock().unwrap() = None;
        Ok(())
    }

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        *self.output.lock().unwrap() = None;
        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.decoder.lock().unwrap() = None;
        *self.output.lock().unwrap() = None;
        Ok(())
    }

    fn set_format(
        &self,
        state: &VideoCodecState<'static, Readable>,
    ) -> Result<(), gst::LoggableError> {
        *self.input_state.lock().unwrap() = Some(state.clone());
        gst::debug!(CAT, imp = self, "Setting format {:?}", state.caps());
        // The output format depends on the stream, so it is negotiated with the first frame.
        *self.output.lock().unwrap() = None;
        Ok(())
    }

    fn sink_query(&self, query: &mut gst::QueryRef) -> bool {
        if let gst::QueryViewMut::Context(ctxquery) = query.view_mut() {
            let device = self.device();
            if gst_hip::hip_handle_context_query(&*self.obj(), ctxquery, device.as_ref()) {
                return true;
            }
        }
        self.parent_sink_query(query)
    }

    fn src_query(&self, query: &mut gst::QueryRef) -> bool {
        if let gst::QueryViewMut::Context(ctxquery) = query.view_mut() {
            let device = self.device();
            if gst_hip::hip_handle_context_query(&*self.obj(), ctxquery, device.as_ref()) {
                return true;
            }
        }
        self.parent_src_query(query)
    }

    fn decide_allocation(
        &self,
        query: &mut gst::query::Allocation,
    ) -> Result<(), gst::LoggableError> {
        let device = self
            .device()
            .ok_or_else(|| gst::loggable_error!(CAT, "No HIP device"))?;

        let (caps, _) = query.get_owned();
        let caps = caps.ok_or_else(|| gst::loggable_error!(CAT, "No caps in the query"))?;
        let info = gst_video::VideoInfo::from_caps(&caps)
            .map_err(|_| gst::loggable_error!(CAT, "Invalid caps {caps:?}"))?;
        let size = info.size() as u32;

        // Reuse a downstream HIP pool: downstream got the device from us through the context
        // query, so it is on the same device.
        let (pool, min, max) = query
            .allocation_pools()
            .find_map(|(pool, _size, min, max)| {
                let pool = pool?;
                pool.is::<gst_hip::HipBufferPool>()
                    .then_some((pool, min, max))
            })
            .unwrap_or_else(|| (gst_hip::HipBufferPool::new(&device).upcast(), 0, 0));

        let mut config = pool.config();
        config.set_params(Some(&caps), size, min, max);
        pool.set_config(config)
            .map_err(|_| gst::loggable_error!(CAT, "Failed to configure the HIP buffer pool"))?;

        if query.allocation_pools().next().is_some() {
            query.set_nth_allocation_pool(0, Some(&pool), size, min, max);
        } else {
            query.add_allocation_pool(Some(&pool), size, min, max);
        }
        Ok(())
    }

    fn handle_frame(
        &self,
        mut frame: gst_video::VideoCodecFrame,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let obj = self.obj();
        let settings = self.settings();
        let device = self.device().ok_or(gst::FlowError::NotNegotiated)?;

        // HIP's current device is thread local: select it on the streaming thread.
        if !device.set_current() {
            gst::element_imp_error!(
                self,
                gst::LibraryError::Failed,
                ["Failed to set the HIP device"]
            );
            return Err(gst::FlowError::Error);
        }

        let input = frame.input_buffer_owned().ok_or(gst::FlowError::Error)?;
        let map = input.map_readable().map_err(|_| {
            gst::element_imp_error!(
                self,
                gst::CoreError::Failed,
                ["Failed to map the input buffer"]
            );
            gst::FlowError::Error
        })?;

        let mut decoder_guard = self.decoder.lock().unwrap();
        if decoder_guard.is_none() {
            let backend = match settings.backend {
                Backend::Hardware => rocjpeg::Backend::Hardware,
                Backend::Hybrid => rocjpeg::Backend::Hybrid,
            };
            let decoder = Decoder::new(backend, device.device_id() as i32).map_err(|err| {
                if err.is_hw_decoder_unsupported() {
                    gst::element_imp_error!(
                        self,
                        gst::LibraryError::Init,
                        ["The hardware JPEG decoder is not supported ({err}), try backend=hybrid"]
                    );
                } else {
                    gst::element_imp_error!(
                        self,
                        gst::LibraryError::Init,
                        ["Failed to create the decoder: {err}"]
                    );
                }
                gst::FlowError::Error
            })?;
            *decoder_guard = Some(decoder);
        }
        // The decoder is only used by the streaming thread, take it out so that the lock is not
        // held while negotiating (which queries the element).
        let decoder = decoder_guard.take().unwrap();
        drop(decoder_guard);
        let result = self.decode(&decoder, &mut frame, map.as_slice());
        *self.decoder.lock().unwrap() = Some(decoder);
        drop(map);

        match result {
            Ok(()) => {
                let _ = obj;
                self.obj().finish_frame(frame)
            }
            Err(DecodeError::Bad(err)) => {
                gst::warning!(CAT, imp = self, "Dropping invalid JPEG: {err}");
                self.obj().drop_frame(frame)
            }
            Err(DecodeError::Flow(flow)) => Err(flow),
        }
    }
}

enum DecodeError {
    /// The input isn't a valid JPEG, drop the frame.
    Bad(rocjpeg::Error),
    Flow(gst::FlowError),
}

impl From<gst::FlowError> for DecodeError {
    fn from(flow: gst::FlowError) -> Self {
        Self::Flow(flow)
    }
}

impl RocJpegDec {
    fn decode(
        &self,
        decoder: &Decoder,
        frame: &mut gst_video::VideoCodecFrame,
        data: &[u8],
    ) -> Result<(), DecodeError> {
        let obj = self.obj();

        let stream = Stream::parse(data).map_err(|err| {
            if err.is_bad_jpeg() {
                DecodeError::Bad(err)
            } else {
                gst::element_imp_error!(
                    self,
                    gst::StreamError::Decode,
                    ["Failed to parse the JPEG: {err}"]
                );
                DecodeError::Flow(gst::FlowError::Error)
            }
        })?;
        let info = decoder.image_info(&stream).map_err(|err| {
            if err.is_bad_jpeg() {
                DecodeError::Bad(err)
            } else {
                gst::element_imp_error!(
                    self,
                    gst::StreamError::Decode,
                    ["Failed to read the JPEG info: {err}"]
                );
                DecodeError::Flow(gst::FlowError::Error)
            }
        })?;

        // (Re)negotiate when the stream changed.
        let wanted = (info.width(), info.height(), info.subsampling);
        let current = *self.output.lock().unwrap();
        let (format, output_format) = match current {
            Some(c) if (c.width, c.height, c.subsampling) == wanted => {
                let (_, output_format) = *candidate_formats(c.subsampling)
                    .iter()
                    .find(|(f, _)| *f == c.format)
                    .expect("negotiated format is a candidate");
                (c.format, output_format)
            }
            _ => {
                let (format, output_format) = self.negotiate_output(&info)?;
                *self.output.lock().unwrap() = Some(OutputConfig {
                    width: wanted.0,
                    height: wanted.1,
                    subsampling: wanted.2,
                    format,
                });
                (format, output_format)
            }
        };

        obj.allocate_output_frame(frame, None)?;
        let buffer = frame.output_buffer_mut().ok_or(gst::FlowError::Error)?;
        let memory = buffer.peek_memory(0);
        let Some(hip_memory) = memory.downcast_memory_ref::<gst_hip::HipMemory>() else {
            gst::element_imp_error!(
                self,
                gst::CoreError::Negotiation,
                ["The output buffer is not HIP memory"]
            );
            return Err(gst::FlowError::Error.into());
        };
        let layout = hip_memory.video_info();
        debug_assert_eq!(layout.format(), format);

        // Map the frame as device memory for writing.
        let mut hip_frame =
            HipVideoFrameRef::from_buffer_ref_writable(buffer, &layout).map_err(|_| {
                gst::element_imp_error!(
                    self,
                    gst::CoreError::Failed,
                    ["Failed to map the HIP memory"]
                );
                gst::FlowError::Error
            })?;

        let mut image = rocjpeg::Image::new();
        for plane in 0..layout.n_planes() {
            let ptr = hip_frame
                .plane_device_ptr_mut(plane)
                .map_err(|_| gst::FlowError::Error)?;
            image.set_plane(plane as usize, ptr, layout.stride()[plane as usize] as u32);
        }

        let result =
            unsafe { decoder.decode(&stream, &DecodeParams::new(output_format), &mut image) };
        drop(hip_frame);

        result.map_err(|err| {
            if err.is_bad_jpeg() {
                DecodeError::Bad(err)
            } else {
                gst::element_imp_error!(
                    self,
                    gst::StreamError::Decode,
                    ["Failed to decode: {err}"]
                );
                DecodeError::Flow(gst::FlowError::Error)
            }
        })
    }
}

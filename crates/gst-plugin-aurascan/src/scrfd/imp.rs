use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use byte_slice_cast::*;
use gst::glib;
use gst::subclass::prelude::*;
use gst_video::prelude::*;
use gst_video::subclass::prelude::*;

use burn::tensor::TensorData;
use burn::{Dispatch, DispatchDevice, Tensor};

use crate::BackendType;
use crate::scrfd::{
    ModelType, SCRFD_BBOX8_OUT_ID, SCRFD_BBOX16_OUT_ID, SCRFD_BBOX32_OUT_ID, SCRFD_GROUP_ID,
    SCRFD_KPS_GROUP_ID, SCRFD_KPS8_OUT_ID, SCRFD_KPS16_OUT_ID, SCRFD_KPS32_OUT_ID,
    SCRFD_SCORE8_OUT_ID, SCRFD_SCORE16_OUT_ID, SCRFD_SCORE32_OUT_ID,
};

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "burn-scrfdinference",
        gst::DebugColorFlags::empty(),
        Some("Burn SCRFD Inference Element"),
    )
});

struct VecWrapper<T>(Vec<T>);

impl<T: ToByteSlice> AsRef<[u8]> for VecWrapper<T> {
    fn as_ref(&self) -> &[u8] {
        self.0.as_byte_slice()
    }
}
impl<T: ToMutByteSlice> AsMut<[u8]> for VecWrapper<T> {
    fn as_mut(&mut self) -> &mut [u8] {
        self.0.as_mut_byte_slice()
    }
}

struct State {
    model: Box<Model>,
    info: Option<gst_video::VideoInfo>,
}

struct Settings {
    backend_type: BackendType,
    model_type: ModelType,
    weights_path: Option<PathBuf>,
    cubecl_type_id: u32,
    cubecl_index_id: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            backend_type: Default::default(),
            model_type: Default::default(),
            weights_path: Default::default(),
            cubecl_type_id: u32::MAX,
            cubecl_index_id: u32::MAX,
        }
    }
}

pub struct ScrfdInference {
    state: Mutex<Option<State>>,
    settings: Mutex<Settings>,
    input_tensor_caps: Mutex<gst::Caps>,
    output_tensor_caps: Mutex<gst::Caps>,
}

impl Default for ScrfdInference {
    fn default() -> Self {
        let mut input_tensor_caps = gst_video::VideoCapsBuilder::new()
            .format(gst_video::VideoFormat::Rgb)
            .pixel_aspect_ratio(gst::Fraction::new(1, 1))
            .build();

        let mut output_tensor_caps = gst_video::VideoCapsBuilder::new()
            .format(gst_video::VideoFormat::Rgb)
            .pixel_aspect_ratio(gst::Fraction::new(1, 1))
            .build();

        set_width_and_height_caps(&mut input_tensor_caps);
        set_width_and_height_caps(&mut output_tensor_caps);

        Self {
            state: Default::default(),
            settings: Default::default(),
            input_tensor_caps: Mutex::new(input_tensor_caps),
            output_tensor_caps: Mutex::new(output_tensor_caps),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ScrfdInference {
    const NAME: &'static str = "GstAscScrfdInference";

    type Type = super::ScrfdInference;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for ScrfdInference {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecEnum::builder("backend-type")
                    .nick("Backend Type")
                    .blurb("Burn backend to use")
                    .default_value(Settings::default().backend_type)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecEnum::builder("model-type")
                    .nick("Model Type")
                    .blurb("SCRFD model type to use")
                    .default_value(Settings::default().model_type)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecString::builder("weights-path")
                    .nick("Weights Path")
                    .blurb("Path to the weights file")
                    .default_value(None)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("cubecl-type-id")
                    .nick("Cubcl Type ID")
                    .blurb("Type ID that identifies the type of the device. For CubeCL backends only, -1 for default.")
                    .default_value(Settings::default().cubecl_type_id)
                    .mutable_ready()
                    .build(),
                glib::ParamSpecUInt::builder("cubecl-index-id")
                    .nick("Cubcl Index ID")
                    .blurb("Index ID that identifies the device number. For CubeCL backends only.")
                    .default_value(Settings::default().cubecl_index_id)
                    .mutable_ready()
                    .build(),
            ]
        });

        &PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "backend-type" => {
                let mut settings = self.settings.lock().unwrap();
                settings.backend_type = value.get().unwrap();
            }
            "model-type" => {
                let mut settings = self.settings.lock().unwrap();
                settings.model_type = value.get().unwrap();
            }
            "weights-path" => {
                let mut settings = self.settings.lock().unwrap();
                settings.weights_path = value.get().unwrap();
            }
            "cubecl-type-id" => {
                let mut settings = self.settings.lock().unwrap();
                settings.cubecl_type_id = value.get().unwrap();
            }
            "cubecl-index-id" => {
                let mut settings = self.settings.lock().unwrap();
                settings.cubecl_index_id = value.get().unwrap();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "backend-type" => {
                let settings = self.settings.lock().unwrap();
                settings.backend_type.to_value()
            }
            "model-type" => {
                let settings = self.settings.lock().unwrap();
                settings.model_type.to_value()
            }
            "weights-path" => {
                let settings = self.settings.lock().unwrap();
                settings.weights_path.to_value()
            }
            "cubecl-type-id" => {
                let settings = self.settings.lock().unwrap();
                settings.cubecl_type_id.to_value()
            }
            "cubecl-index-id" => {
                let settings = self.settings.lock().unwrap();
                settings.cubecl_index_id.to_value()
            }
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for ScrfdInference {}

impl ElementImpl for ScrfdInference {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Burn SCRFD Inference Element",
                "Filter/Video",
                "Runs inference on video frames via the SCRFD model",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });
        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let mut sink_caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .pixel_aspect_ratio(gst::Fraction::new(1, 1))
                .build();
            set_width_and_height_caps(&mut sink_caps);

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &sink_caps,
            )
            .unwrap();

            let mut src_caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .pixel_aspect_ratio(gst::Fraction::new(1, 1))
                .build();
            set_width_and_height_caps(&mut src_caps);

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

impl BaseTransformImpl for ScrfdInference {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn start(&self) -> Result<(), gst::ErrorMessage> {
        let mut state = self.state.lock().unwrap();
        let settings = self.settings.lock().unwrap();

        let model = match Model::load_model(&settings) {
            Ok(model) => model,
            Err(err) => {
                gst::error!(CAT, imp = self, "Failed to load model: {err}");
                return Err(gst::error_msg!(
                    gst::LibraryError::Settings,
                    ["Failed to load model: {err}"]
                ));
            }
        };
        *state = Some(State { model, info: None });

        let state = state.as_mut().unwrap();

        let strided = |tensor_id: &glib::GStr, channels: i32| {
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
        };

        let v_tensor_s = if state.model.is_kps() {
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

        let group_id = if state.model.is_kps() {
            SCRFD_KPS_GROUP_ID
        } else {
            SCRFD_GROUP_ID
        };

        let mut tensor_s = gst::Structure::new_empty("tensorgroups");
        tensor_s.set(group_id, v_tensor_s);

        let mut output_tensor_caps = self.output_tensor_caps.lock().unwrap();
        output_tensor_caps.make_mut().set("tensors", tensor_s);

        gst::info!(CAT, imp = self, "Started");

        Ok(())
    }

    fn stop(&self) -> Result<(), gst::ErrorMessage> {
        *self.state.lock().unwrap() = None;
        gst::info!(CAT, imp = self, "Stopped");
        Ok(())
    }

    fn set_caps(&self, incaps: &gst::Caps, outcaps: &gst::Caps) -> Result<(), gst::LoggableError> {
        gst::debug!(CAT, imp = self, "Received caps {incaps:?}");

        let mut state_guard = self.state.lock().unwrap();
        let Some(state) = &mut *state_guard else {
            return Err(gst::loggable_error!(CAT, "Invalid state"));
        };

        let Ok(info) = gst_video::VideoInfo::from_caps(incaps) else {
            return Err(gst::loggable_error!(CAT, "Invalid caps"));
        };
        state.info = Some(info);
        drop(state_guard);

        self.parent_set_caps(incaps, outcaps)
    }

    fn transform_caps(
        &self,
        direction: gst::PadDirection,
        caps: &gst::Caps,
        filter: Option<&gst::Caps>,
    ) -> Option<gst::Caps> {
        let restrictions = self.input_tensor_caps.lock().unwrap();

        match direction {
            gst::PadDirection::Src => {
                // Translate src pad caps → sink pad caps: strip tensors so they
                // don't propagate upstream, then apply video format constraints.
                let mut res = caps.copy();
                for s in res.get_mut().unwrap().iter_mut() {
                    s.remove_field("tensors");
                }
                let res = res.intersect_with_mode(&restrictions, gst::CapsIntersectMode::First);
                let res = filter
                    .map(|f| f.intersect_with_mode(&res, gst::CapsIntersectMode::First))
                    .unwrap_or(res);
                Some(res)
            }
            _ => {
                // Translate sink pad caps → src pad caps: take the incoming video
                // caps and set the exact tensor group the loaded model produces.
                //
                // Intersection cannot be used here because `caps` (plain video)
                // lacks the `tensors` field — in First-mode intersection, absent
                // fields are dropped, so the tensor spec would be lost and the
                // downstream filter's groups would bleed through unchanged.
                let output_tensor_caps = self.output_tensor_caps.lock().unwrap();
                let tensor_s = output_tensor_caps
                    .structure(0)
                    .and_then(|s| s.get::<gst::Structure>("tensors").ok());

                let mut res = caps.copy();
                if let Some(ref tensor_s) = tensor_s {
                    for s in res.get_mut().unwrap().iter_mut() {
                        s.set("tensors", tensor_s.clone());
                    }
                }

                // Intersect with the filter for video-level constraints only.
                // Strip tensor fields from the filter first — the filter may
                // carry a combined structure with all pipeline groups, and
                // GstStructure equality-based comparison would make intersection
                // fail against our single-group spec.
                let res = if let Some(filter) = filter {
                    let mut filter_video = filter.copy();
                    for s in filter_video.get_mut().unwrap().iter_mut() {
                        s.remove_field("tensors");
                        s.remove_field("tensorgroups");
                    }
                    let mut res_video = res.copy();
                    for s in res_video.get_mut().unwrap().iter_mut() {
                        s.remove_field("tensors");
                    }
                    let mut intersected =
                        filter_video.intersect_with_mode(&res_video, gst::CapsIntersectMode::First);
                    if let Some(ref tensor_s) = tensor_s {
                        for s in intersected.get_mut().unwrap().iter_mut() {
                            s.set("tensors", tensor_s.clone());
                        }
                    }
                    intersected
                } else {
                    res
                };

                Some(res)
            }
        }
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

        let Some(ref info) = state.info else {
            gst::error!(CAT, imp = self, "No caps");
            return Err(gst::FlowError::NotNegotiated);
        };

        let Ok(frame) = gst_video::VideoFrameRef::from_buffer_ref_writable(buffer, info) else {
            gst::error!(CAT, imp = self, "Failed to map video frame");
            return Err(gst::FlowError::Error);
        };

        let width = frame.width() as usize;
        let height = frame.height() as usize;
        let stride = frame.plane_stride()[0] as usize;

        let mut input = vec![0u8; width * height * 3];
        let in_data = frame.plane_data(0).unwrap();

        for (out_line, in_line) in Iterator::zip(
            input.chunks_exact_mut(width * 3),
            in_data.chunks_exact(stride),
        ) {
            out_line.copy_from_slice(&in_line[..width * 3]);
        }
        drop(frame);

        let output = state.model.forward(input, width, height);

        let into_gst_tensor = |name: &'static glib::GStr, burn_tensor: Tensor<Dispatch, 3>| {
            let dims = burn_tensor.dims();
            let tensor_data = burn_tensor.into_data().into_vec::<f32>().unwrap();
            let tensor_data = gst::Buffer::from_slice(VecWrapper(tensor_data));
            gst_analytics::Tensor::new_simple(
                glib::Quark::from_static_str(name),
                gst_analytics::TensorDataType::Float32,
                tensor_data,
                gst_analytics::TensorDimOrder::RowMajor,
                &dims,
            )
        };

        let mut tensors = Vec::new();
        let mut output = output.into_iter();
        tensors.push(into_gst_tensor(SCRFD_SCORE8_OUT_ID, output.next().unwrap()));
        tensors.push(into_gst_tensor(
            SCRFD_SCORE16_OUT_ID,
            output.next().unwrap(),
        ));
        tensors.push(into_gst_tensor(
            SCRFD_SCORE32_OUT_ID,
            output.next().unwrap(),
        ));
        tensors.push(into_gst_tensor(SCRFD_BBOX8_OUT_ID, output.next().unwrap()));
        tensors.push(into_gst_tensor(SCRFD_BBOX16_OUT_ID, output.next().unwrap()));
        tensors.push(into_gst_tensor(SCRFD_BBOX32_OUT_ID, output.next().unwrap()));
        if state.model.is_kps() {
            tensors.push(into_gst_tensor(SCRFD_KPS8_OUT_ID, output.next().unwrap()));
            tensors.push(into_gst_tensor(SCRFD_KPS16_OUT_ID, output.next().unwrap()));
            tensors.push(into_gst_tensor(SCRFD_KPS32_OUT_ID, output.next().unwrap()));
        }

        let mut meta = gst_analytics::TensorMeta::add(buffer);
        meta.set(glib::Slice::from_iter(tensors));

        Ok(gst::FlowSuccess::Ok)
    }
}

fn set_width_and_height_caps(caps: &mut gst::Caps) {
    let caps = caps.get_mut().unwrap();
    caps.set("width", gst::IntRange::with_step(32, i32::MAX - 31, 32));
    caps.set("height", gst::IntRange::with_step(32, i32::MAX - 31, 32));
}

struct Model {
    model: scrfd_burn::Model<Dispatch>,
    device: DispatchDevice,
}

impl Model {
    fn is_kps(&self) -> bool {
        self.model.is_kps()
    }

    fn load_model(settings: &Settings) -> eyre::Result<Box<Self>> {
        let device = match settings.backend_type {
            BackendType::Flex => DispatchDevice::Flex(Default::default()),
            #[cfg(feature = "vulkan")]
            BackendType::Vulkan => {
                use burn::tensor::backend::{Device, DeviceId};
                match (settings.cubecl_type_id, settings.cubecl_index_id) {
                    (u32::MAX, _) => DispatchDevice::Vulkan(Default::default()),
                    (type_id, index_id) => DispatchDevice::from_id(DeviceId {
                        type_id: type_id as u16,
                        index_id: index_id as u16,
                    }),
                }
            }
            #[cfg(feature = "rocm")]
            BackendType::Rocm => {
                use burn::tensor::backend::{Device, DeviceId};
                match (settings.cubecl_type_id, settings.cubecl_index_id) {
                    (u32::MAX, _) => DispatchDevice::Rocm(Default::default()),
                    (type_id, index_id) => DispatchDevice::from_id(DeviceId {
                        type_id: type_id as u16,
                        index_id: index_id as u16,
                    }),
                }
            }
        };

        Self::load_model_internal(settings, device)
    }

    #[allow(unused_variables)]
    fn load_model_internal(settings: &Settings, device: DispatchDevice) -> eyre::Result<Box<Self>> {
        match settings.weights_path {
            Some(_) => unimplemented!(),
            None => {
                #[cfg(feature = "scrfd-embedded")]
                {
                    Ok(Box::new(Self {
                        model: scrfd_burn::Model::from_embedded(
                            settings.model_type.into(),
                            &device,
                        ),
                        device,
                    }))
                }
                #[cfg(not(feature = "scrfd-embedded"))]
                {
                    eyre::bail!("Compiled without support for embedded weights")
                }
            }
        }
    }

    fn forward(&self, input: Vec<u8>, width: usize, height: usize) -> Vec<Tensor<Dispatch, 3>> {
        let data = TensorData::new(input, [1, height, width, 3]).convert::<f32>();
        let tensor = Tensor::<Dispatch, 4>::from_data(data, &self.device)
            .permute([0, 3, 1, 2])
            .sub_scalar(127.5f32)
            .div_scalar(128.0f32);
        self.model.forward(tensor)
    }
}

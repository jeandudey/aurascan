use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};

use byte_slice_cast::*;
use gst::glib::value::{ToSendValue, ToValue};
use gst::glib::{self, ParamSpecBuilderExt};
use gst::prelude::GstParamSpecBuilderExt;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::*;
use gst_video::prelude::*;

use burn::tensor::TensorData;
use burn::{Dispatch, DispatchDevice, Tensor};
use sixdrepnet360_burn::sixdrepnet360;

use eyre::Context;

use crate::BackendType;

const GROUP_ID: &glib::GStr = glib::gstr!("sixdrepnet360");
const SIXDREPNET360_TENSOR_ID: &glib::GStr = glib::gstr!("sixdrepnet360-out");

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "burn-sixdrepnet360inference",
        gst::DebugColorFlags::empty(),
        Some("Burn 6DRepNet360 Inference Element"),
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
    weights_path: Option<PathBuf>,
    cubecl_type_id: u32,
    cubecl_index_id: u32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            backend_type: Default::default(),
            weights_path: None,
            cubecl_type_id: u32::MAX,
            cubecl_index_id: u32::MAX,
        }
    }
}

pub struct SixDRepNet360Inference {
    state: Mutex<Option<State>>,
    settings: Mutex<Settings>,
}

impl Default for SixDRepNet360Inference {
    fn default() -> Self {
        Self {
            state: Mutex::new(None),
            settings: Mutex::new(Settings::default()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SixDRepNet360Inference {
    const NAME: &'static str = "GstBurnExtraSixDRepNet360Inference";

    type Type = super::SixDRepNet360Inference;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for SixDRepNet360Inference {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecEnum::builder("backend-type")
                    .nick("Backend Type")
                    .blurb("Burn backend to use")
                    .default_value(Settings::default().backend_type)
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

impl GstObjectImpl for SixDRepNet360Inference {}

impl ElementImpl for SixDRepNet360Inference {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static ELEMENT_METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Burn 6DRepNet360 Inference Element",
                "Filter/Video",
                "Runs inference on video frames via the 6DRepNet360 model",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*ELEMENT_METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let sink_caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .width(224)
                .height(224)
                .build();

            let sink_pad_template = gst::PadTemplate::new(
                "sink",
                gst::PadDirection::Sink,
                gst::PadPresence::Always,
                &sink_caps,
            )
            .unwrap();

            let src_caps = gst_video::VideoCapsBuilder::new()
                .format(gst_video::VideoFormat::Rgb)
                .width(224)
                .height(224)
                .field(
                    "tensors",
                    gst::Structure::builder("tensorgroups")
                        .field(
                            GROUP_ID,
                            gst::UniqueList::new([gst::Caps::builder("tensor/strided")
                                .field("field-id", SIXDREPNET360_TENSOR_ID)
                                .field(
                                    "dims",
                                    gst::Array::from_values([
                                        1i32.to_send_value(),
                                        3i32.to_send_value(),
                                        3i32.to_send_value(),
                                    ]),
                                )
                                .field("dims-order", "row-major")
                                .field("type", "float32")
                                .build()]),
                        )
                        .build(),
                )
                .build();

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

impl BaseTransformImpl for SixDRepNet360Inference {
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
        let res = if direction == gst::PadDirection::Src {
            let mut res = caps.copy();
            for s in res.get_mut().unwrap().iter_mut() {
                if let Ok(mut tensors) = s.get::<gst::Structure>("tensors") {
                    tensors.remove_field(GROUP_ID);
                    s.set("tensors", tensors);
                }
            }

            res
        } else {
            let mut res = caps.copy();
            for s in res.get_mut().unwrap().iter_mut() {
                let mut tensors = s
                    .get::<gst::Structure>("tensors")
                    .ok()
                    .unwrap_or_else(|| gst::Structure::new_empty("tensorgroups"));

                tensors.set(
                    GROUP_ID,
                    gst::UniqueList::new([gst::Caps::builder("tensor/strided")
                        .field("tensor-id", SIXDREPNET360_TENSOR_ID)
                        .field(
                            "dims",
                            gst::Array::from_values([
                                1i32.to_send_value(),
                                3i32.to_send_value(),
                                3i32.to_send_value(),
                            ]),
                        )
                        .field("dims-order", "row-major")
                        .field("type", "float32")
                        .build()]),
                );

                s.set("tensors", tensors);
            }

            res
        };

        let res = filter
            .map(|filter| filter.intersect_with_mode(&res, gst::CapsIntersectMode::First))
            .unwrap_or(res);

        Some(res)
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

        let dims = output.dims();
        let tensor_data = output.into_data().into_vec::<f32>().unwrap();
        let tensor_data = gst::Buffer::from_slice(VecWrapper(tensor_data));
        let tensor = gst_analytics::Tensor::new_simple(
            glib::Quark::from_static_str(SIXDREPNET360_TENSOR_ID),
            gst_analytics::TensorDataType::Float32,
            tensor_data,
            gst_analytics::TensorDimOrder::RowMajor,
            &dims,
        );

        let mut meta = gst_analytics::TensorMeta::add(buffer);
        meta.set(glib::Slice::from_iter([tensor]));

        Ok(gst::FlowSuccess::Ok)
    }
}

struct Model {
    model: sixdrepnet360::SixDRepNet360<Dispatch>,
    std: Tensor<Dispatch, 4>,
    mean: Tensor<Dispatch, 4>,
    device: DispatchDevice,
}

impl Model {
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
        let mean =
            Tensor::<Dispatch, 1>::from_data([0.485, 0.485, 0.406], &device).reshape([1, 3, 1, 1]);
        let std =
            Tensor::<Dispatch, 1>::from_data([0.229, 0.224, 0.225], &device).reshape([1, 3, 1, 1]);
        match &settings.weights_path {
            Some(torch_weights) => Ok(Box::new(Self {
                model: sixdrepnet360::SixDRepNet360::from_file(&torch_weights, &device)
                    .wrap_err("Failed to load PyTorch weights for 6DRepNet360")?,
                mean,
                std,
                device,
            })),
            None => {
                #[cfg(feature = "sixdrepnet360-pretrained")]
                {
                    Ok(Box::new(Self {
                        model: sixdrepnet360::SixDRepNet360::pretrained(&device)
                            .wrap_err("Failed to load pretrained weights for 6DRepNet360")?,
                        mean,
                        std,
                        device,
                    }))
                }
                #[cfg(not(feature = "sixdrepnet360-pretrained"))]
                {
                    eyre::bail!("Compiled without support for pretrained weights")
                }
            }
        }
    }

    fn forward(&self, input: Vec<u8>, width: usize, height: usize) -> Tensor<Dispatch, 3> {
        let data = TensorData::new(input, [1, height, width, 3]).convert::<f32>();
        let tensor = Tensor::from_data(data, &self.device)
            .permute([0, 3, 1, 2])
            .div_scalar(255.0)
            .sub(self.mean.clone())
            .div(self.std.clone());
        self.model.forward(tensor)
    }
}

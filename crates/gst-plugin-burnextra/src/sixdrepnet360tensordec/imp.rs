use byte_slice_cast::AsSliceOf;
use gst::glib;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::BaseTransformImpl;
use gst_video::prelude::*;
use std::sync::LazyLock;

const GROUP_ID: &glib::GStr = glib::gstr!("sixdrepnet360");
const SIXDREPNET360_TENSOR_ID: &glib::GStr = glib::gstr!("sixdrepnet360-out");

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "sixdrepnet360tensordec",
        gst::DebugColorFlags::empty(),
        Some("SCRFD Tensor Decoder"),
    )
});

#[derive(Debug)]
struct Angle {
    pub pitch: f32,
    pub yaw: f32,
    pub roll: f32,
}

#[derive(Default)]
pub struct SixDRepNet360TensorDec;

#[glib::object_subclass]
impl ObjectSubclass for SixDRepNet360TensorDec {
    const NAME: &'static str = "GstBurnExtraSixDRepNet360TensorDec";

    type Type = super::SixDRepNet360TensorDec;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for SixDRepNet360TensorDec {}

impl GstObjectImpl for SixDRepNet360TensorDec {}

impl ElementImpl for SixDRepNet360TensorDec {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "6DRepNet360 Tensor Decoder",
                "Tensordecoder/Video",
                "Decodes tensors from 6DRepNet360 model",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let sink_caps = gst_video::VideoCapsBuilder::new()
                .field(
                    "tensorgroups",
                    gst::Structure::builder("tensors")
                        .field(
                            GROUP_ID,
                            gst::UniqueList::new([gst::Caps::builder("tensor/strided")
                                .field("tensor-id", SIXDREPNET360_TENSOR_ID)
                                .field(
                                    "dims",
                                    gst::Array::from_values([
                                        0i32.to_send_value(),
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

impl BaseTransformImpl for SixDRepNet360TensorDec {
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

    fn set_caps(
        &self,
        _incaps: &gst::Caps,
        _outcaps: &gst::Caps,
    ) -> Result<(), gst::LoggableError> {
        Ok(())
    }

    fn transform_ip(
        &self,
        buffer: &mut gst::BufferRef,
    ) -> Result<gst::FlowSuccess, gst::FlowError> {
        let mut angles = Vec::new();
        for meta in buffer.iter_meta::<gst_analytics::TensorMeta>() {
            let Some(tensor) = find_tensor(&meta) else {
                continue;
            };

            let Some(map) = tensor.data().map_readable().ok() else {
                continue;
            };

            let Some(data) = map.as_slice_of::<f32>().ok() else {
                continue;
            };
            let n = data.len() / 9;

            for i in 0..n {
                let r = &data[i * 9..i * 9 + 9];

                let (r00, r10, r20, r21, r22) = (r[0], r[3], r[6], r[7], r[8]);

                let sy = (r00 * r00 + r10 * r10).sqrt();
                let singular = sy < 1e-6;

                let angle = if !singular {
                    Angle {
                        pitch: r21.atan2(r22),
                        yaw: (-r20).atan2(sy),
                        roll: r10.atan2(r00),
                    }
                } else {
                    Angle {
                        pitch: (-r[5]).atan2(r[4]),
                        yaw: (-r20).atan2(sy),
                        roll: 0.0,
                    }
                };

                angles.push(Angle {
                    pitch: angle.pitch.to_degrees(),
                    yaw: angle.yaw.to_degrees(),
                    roll: angle.roll.to_degrees(),
                });
            }
        }

        for angle in angles.into_iter() {
            add_angle(buffer, angle);
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

fn add_angle(buffer: &mut gst::BufferRef, angle: Angle) {
    let mut meta = gst::meta::CustomMeta::add(buffer, "EulerAnglesMeta").unwrap();
    let s = meta.mut_structure();
    s.set("yaw", angle.yaw);
    s.set("pitch", angle.pitch);
    s.set("roll", angle.roll);
}

fn find_tensor<'a>(
    meta: &'a gst::MetaRef<'a, gst_analytics::TensorMeta>,
) -> Option<&'a gst_analytics::Tensor> {
    meta.typed_tensor(
        glib::Quark::from_static_str(SIXDREPNET360_TENSOR_ID),
        gst_analytics::TensorDataType::Float32,
        gst_analytics::TensorDimOrder::RowMajor,
        &[usize::MAX, 3, 3],
    )
}

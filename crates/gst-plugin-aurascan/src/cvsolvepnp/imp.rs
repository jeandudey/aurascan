use std::sync::{LazyLock, Mutex};

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::BaseTransformImpl;

use opencv::calib3d;
use opencv::core::{CV_64F, Point2f, Point3f, Vector};
use opencv::prelude::*;

use eyre::Context;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "cvsolvepnp",
        gst::DebugColorFlags::empty(),
        Some("SolvePnP Element"),
    )
});

#[derive(Debug, Default)]
pub struct SolvePnp {
    intrinsics: Mutex<Intrinsics>,
}

impl SolvePnp {
    fn solve(
        &self,
        object_points: &Vector<Point3f>,
        image_points: &Vector<Point2f>,
        flags: i32,
    ) -> eyre::Result<(Mat, Mat)> {
        let intrinsics = self.intrinsics.lock().unwrap();
        if !intrinsics.is_configured() {
            eyre::bail!("No camera intrinsics configured");
        }

        let camera_matrix = intrinsics
            .camera_matrix()
            .wrap_err("Failed to construct camera matrix")?;

        let dist_coeffs = &intrinsics
            .dist_coeffs()
            .wrap_err("Failed to construct distortion coefficients matrix")?;

        let mut rvec = Mat::default();
        let mut tvec = Mat::default();

        let ok = calib3d::solve_pnp(
            object_points,
            image_points,
            &camera_matrix,
            &dist_coeffs,
            &mut rvec,
            &mut tvec,
            // TODO: No initial guess supported, I guess this could improve accuracy
            // by giving an initial rotation from inference results.
            false,
            flags,
        )
        .wrap_err("Failed to run solve_pnp")?;
        if !ok {
            eyre::bail!("solve_pnp returned false");
        }

        Ok((tvec, rvec))
    }
}

#[glib::object_subclass]
impl ObjectSubclass for SolvePnp {
    const NAME: &'static str = "GstAscSolvePnp";
    type Type = super::CvSolvePnp;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for SolvePnp {
    fn properties() -> &'static [glib::ParamSpec] {
        static PROPERTIES: LazyLock<Vec<glib::ParamSpec>> = LazyLock::new(|| {
            vec![
                glib::ParamSpecDouble::builder("fx")
                    .nick("Focal Point X")
                    .blurb("The x coordinate of the focal point, in pixels")
                    .default_value(0.0)
                    .mutable_playing()
                    .build(),
                glib::ParamSpecDouble::builder("fy")
                    .nick("Focal Point Y")
                    .blurb("The y coordinate of the focal point, in pixels")
                    .default_value(0.0)
                    .mutable_playing()
                    .build(),
                glib::ParamSpecDouble::builder("cx")
                    .nick("Center Point X")
                    .blurb("The x coordinate of the center point, in pixels")
                    .default_value(0.0)
                    .mutable_playing()
                    .build(),
                glib::ParamSpecDouble::builder("cy")
                    .nick("Center Point Y")
                    .blurb("The y coordinate of the center point, in pixels")
                    .default_value(0.0)
                    .mutable_playing()
                    .build(),
            ]
        });

        &PROPERTIES
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "fx" => {
                self.intrinsics.lock().unwrap().fx = value.get().unwrap();
            }
            "fy" => {
                self.intrinsics.lock().unwrap().fy = value.get().unwrap();
            }
            "cx" => {
                self.intrinsics.lock().unwrap().cx = value.get().unwrap();
            }
            "cy" => {
                self.intrinsics.lock().unwrap().cy = value.get().unwrap();
            }
            _ => unimplemented!(),
        }
    }

    fn property(&self, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "fx" => self.intrinsics.lock().unwrap().fx.to_value(),
            "fy" => self.intrinsics.lock().unwrap().fy.to_value(),
            "cx" => self.intrinsics.lock().unwrap().cx.to_value(),
            "cy" => self.intrinsics.lock().unwrap().cy.to_value(),
            _ => unimplemented!(),
        }
    }
}

impl GstObjectImpl for SolvePnp {}

impl ElementImpl for SolvePnp {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "OpenCV SolvePnP",
                "Filter/Effect/Video",
                "Finds an object pose from 3D-2D point correspondences using OpenCV's solvePnP",
                "Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>",
            )
        });

        Some(&*METADATA)
    }

    fn pad_templates() -> &'static [gst::PadTemplate] {
        static PAD_TEMPLATES: LazyLock<Vec<gst::PadTemplate>> = LazyLock::new(|| {
            let caps = gst_video::VideoCapsBuilder::new().build();

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

        PAD_TEMPLATES.as_ref()
    }
}

impl BaseTransformImpl for SolvePnp {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn transform_ip(&self, buf: &mut gst::BufferRef) -> Result<gst::FlowSuccess, gst::FlowError> {
        gst::debug!(CAT, imp = self, "SolvePnp::transform_ip");

        let Ok(meta) = gst::meta::CustomMeta::from_buffer(buf, "PnpProblemMeta") else {
            return Ok(gst::FlowSuccess::Ok);
        };

        let Ok(problem) = PnpProblemMeta::try_from(meta).inspect_err(|err| {
            gst::error!(CAT, imp = self, "Failed to parse PnpProblemMeta: {err}");
        }) else {
            return Ok(gst::FlowSuccess::Ok);
        };

        // TODO: Flags.
        if let Err(err) = self.solve(&problem.object_points, &problem.image_points, 0) {
            gst::error!(CAT, imp = self, "Failed to solve PNP: {err}");
        }

        Ok(gst::FlowSuccess::Ok)
    }
}

#[derive(Debug, Default)]
struct Intrinsics {
    /// Focal length x, in pixels.
    pub fx: f64,
    /// Focal length y, in pixels.
    pub fy: f64,
    /// Principal point x, in pixels.
    pub cx: f64,
    /// Principal point y, in pixels.
    pub cy: f64,
    /// Distortion coefficients.
    ///
    /// There is not a way to configure this yet.
    pub dist_coeffs: Vec<f64>,
}

impl Intrinsics {
    pub fn is_configured(&self) -> bool {
        self.fx > 0.0 && self.fy > 0.0 && self.cx > 0.0 && self.cy > 0.0 && self.dist_coeffs_valid()
    }

    pub fn dist_coeffs_valid(&self) -> bool {
        matches!(self.dist_coeffs.len(), 0 | 4 | 5 | 8 | 12 | 14)
    }

    pub fn camera_matrix(&self) -> opencv::Result<Mat> {
        Mat::from_slice_2d(&[
            [self.fx, 0.0, self.cx],
            [0.0, self.fy, self.cy],
            [0.0, 0.0, 1.0],
        ])
    }

    pub fn dist_coeffs(&self) -> opencv::Result<Mat> {
        if self.dist_coeffs.is_empty() {
            Mat::zeros(4, 1, CV_64F).and_then(|m| m.to_mat())
        } else {
            Mat::from_slice(&self.dist_coeffs).and_then(|m| m.try_clone())
        }
    }
}

#[derive(Debug)]
pub struct PnpProblemMeta {
    pub id: u32,
    pub object_points: Vector<Point3f>,
    pub image_points: Vector<Point2f>,
}

impl TryFrom<gst::MetaRef<'_, gst::meta::CustomMeta>> for PnpProblemMeta {
    type Error = eyre::Report;

    fn try_from(meta: gst::MetaRef<'_, gst::meta::CustomMeta>) -> Result<Self, Self::Error> {
        if !meta.has_name("PnpProblemMeta") {
            eyre::bail!("Not a PnpProblemMeta");
        }

        let structure = meta.structure();

        let id: u32 = structure
            .get("id")
            .wrap_err("Failed to get ID from PnpProblemMeta")?;

        Ok(Self {
            id,
            object_points: Vector::default(),
            image_points: Vector::default(),
        })
    }
}

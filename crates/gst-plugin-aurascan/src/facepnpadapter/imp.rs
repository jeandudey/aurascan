use std::sync::LazyLock;

use gst::glib;
use gst::prelude::*;
use gst::subclass::prelude::*;
use gst_base::subclass::prelude::BaseTransformImpl;

static CAT: LazyLock<gst::DebugCategory> = LazyLock::new(|| {
    gst::DebugCategory::new(
        "facepnpadapter",
        gst::DebugColorFlags::empty(),
        Some("FacePnpAdapter Element"),
    )
});

#[derive(Debug, Default)]
pub struct FacePnpAdapter;

#[glib::object_subclass]
impl ObjectSubclass for FacePnpAdapter {
    const NAME: &'static str = "GstAscFacePnpAdapter";
    type Type = super::FacePnpAdapter;
    type ParentType = gst_base::BaseTransform;
}

impl ObjectImpl for FacePnpAdapter {}
impl GstObjectImpl for FacePnpAdapter {}

impl ElementImpl for FacePnpAdapter {
    fn metadata() -> Option<&'static gst::subclass::ElementMetadata> {
        static METADATA: LazyLock<gst::subclass::ElementMetadata> = LazyLock::new(|| {
            gst::subclass::ElementMetadata::new(
                "Face solvePnP Adapter",
                "Filter/Effect/Video",
                "Adapts face selection metadata to metadata for cvsolvepnp",
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

impl BaseTransformImpl for FacePnpAdapter {
    const MODE: gst_base::subclass::BaseTransformMode =
        gst_base::subclass::BaseTransformMode::AlwaysInPlace;
    const PASSTHROUGH_ON_SAME_CAPS: bool = false;
    const TRANSFORM_IP_ON_PASSTHROUGH: bool = true;

    fn transform_ip(&self, buf: &mut gst::BufferRef) -> Result<gst::FlowSuccess, gst::FlowError> {
        let (maybe_id, _location) = match parse_selected_face_meta(buf) {
            Ok(v) => v,
            Err(err) => {
                gst::debug!(CAT, imp = self, "No selected face meta found: {err}");
                return Ok(gst::FlowSuccess::Ok);
            }
        };

        let Ok(mut pnp_problem_meta) = gst::meta::CustomMeta::add(buf, "PnpProblemMeta") else {
            gst::error!(CAT, imp = self, "Failed to add pnp problem meta");
            return Ok(gst::FlowSuccess::Ok);
        };

        let structure = pnp_problem_meta.mut_structure();

        if let Some(id) = maybe_id {
            structure.set("id", id);
        }

        // TODO: Need to get keypoint metadata too. :-)

        Ok(gst::FlowSuccess::Ok)
    }
}

fn parse_selected_face_meta(
    buf: &gst::BufferRef,
) -> eyre::Result<(Option<u64>, gst_analytics::AnalyticsODLocation)> {
    let meta = gst::meta::CustomMeta::from_buffer(buf, "SelectedFaceMeta")?;
    let structure = meta.structure();
    let id = structure.get::<u64>("id").ok();
    let x = structure.get::<i32>("x")?;
    let y = structure.get::<i32>("x")?;
    let w = structure.get::<i32>("w")?;
    let h = structure.get::<i32>("h")?;
    let loc_conf_lvl = structure.get::<f32>("loc-conf-lvl")?;

    Ok((
        id,
        gst_analytics::AnalyticsODLocation {
            x,
            y,
            w,
            h,
            loc_conf_lvl,
        },
    ))
}

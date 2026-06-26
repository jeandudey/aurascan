use std::sync::LazyLock;

use gst::glib;
use gst::glib::object::Cast;
use gst::prelude::*;
use gst::subclass::prelude::*;

#[derive(Default)]
pub struct HeadPoseInferenceBin {}

#[glib::object_subclass]
impl ObjectSubclass for HeadPoseInferenceBin {
    const NAME: &'static str = "GstAscHeadPoseInferenceBin";

    type Type = super::HeadPoseInferenceBin;
    type ParentType = gst::Bin;
    type Interfaces = (gst::ChildProxy,);
}

impl ObjectImpl for HeadPoseInferenceBin {
    fn constructed(&self) {
        self.parent_constructed();

        let obj = self.obj();
        let bin = obj.upcast_ref::<gst::Bin>();

        let facedetectorinfernce = gst::ElementFactory::make("burn-scrfdinference")
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
        let faceselector = gst::ElementFactory::make("faceselector")
            .name("faceselector")
            .build()
            .unwrap();
        let videocropscale = gst::ElementFactory::make("videocropscale")
            .name("videocropscale")
            .build()
            .unwrap();
        let headposeinference = gst::ElementFactory::make("burn-sixdrepnet360inference")
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
            &faceselector,
            &videocropscale,
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

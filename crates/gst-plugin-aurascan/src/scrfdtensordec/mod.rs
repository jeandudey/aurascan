use gst::glib;
use gst::prelude::*;

pub mod imp;

glib::wrapper! {
    /// Decodes the output tensors from the SCRFD model.
    ///
    /// It attaches object detection metadata to the output buffer with "face" as the
    /// metadata classification.
    ///
    /// ## Usage
    ///
    /// This element should be after any inference elements providing the SCRFD tensor
    /// outputs, without any scaling elements after, as the decoding depends on the
    /// width and height of the video frames.
    ///
    /// ## Properties
    ///
    /// #### `score-threshold`
    ///
    /// The minimum score a face detection must have to be included in the metadata.
    ///
    /// Readable | Writable
    ///
    /// #### `iou-threshold`
    ///
    /// The cutoff value for non-maximum suppression between two overlapping
    /// bounding boxes to determine if they are the same face. If two bounding
    /// boxes have an IoU (Intersection over Union) greater than this value,
    /// they are considered the same face, and the lower score box is discarded.
    ///
    /// #### `max-detections`
    ///
    /// The maximum number of face detections to include in the metadata.
    ///
    /// Readable | Writable
    pub struct ScrfdTensorDec(ObjectSubclass<imp::ScrfdTensorDec>) @extends gst_base::BaseTransform, gst::Element, gst::Object;
}

pub fn register(plugin: &gst::Plugin) -> Result<(), glib::BoolError> {
    gst::Element::register(
        Some(plugin),
        "scrfdtensordec",
        gst::Rank::NONE,
        ScrfdTensorDec::static_type(),
    )
}

#![allow(clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_cfg))]

pub use ffi;
pub use glib;
pub use gst;
pub use gst_base;
pub use gst_video;

macro_rules! assert_initialized_main_thread {
    () => {
        if !gst::INITIALIZED.load(std::sync::atomic::Ordering::SeqCst) {
            gst::assert_initialized();
        }
    };
}

macro_rules! skip_assert_initialized {
    () => {};
}

#[allow(
    clippy::too_many_arguments,
    clippy::type_complexity,
    clippy::missing_safety_doc
)]
mod auto;
pub use crate::auto::*;

mod functions;
pub use crate::functions::*;
mod hip_stream;
pub use crate::hip_stream::*;
mod hip_video_frame;
pub use crate::hip_video_frame::*;
mod hip_memory;
pub use crate::hip_memory::*;

pub mod prelude {
    pub use glib::prelude::*;
    pub use gst::prelude::*;

    pub use crate::auto::traits::*;
    pub use crate::hip_video_frame::{HipVideoFrameExt, IsHipVideoFrame};
}

#[cfg(test)]
mod tests {
    use glib::prelude::*;

    #[test]
    fn types_registered() {
        gst::init().unwrap();
        assert_eq!(crate::HipDevice::static_type().name(), "GstHipDevice");
        assert_eq!(crate::HipStream::static_type().name(), "GstHipStream");
        assert_eq!(crate::HipVendor::static_type().name(), "GstHipVendor");
    }
}

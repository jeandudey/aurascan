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

#[allow(clippy::too_many_arguments, clippy::type_complexity, clippy::missing_safety_doc)]
mod auto;
pub use auto::*;

mod functions;
pub use functions::*;

mod hip_stream;
mod hip_video_frame;

// GstObject based types are thread safe.
unsafe impl Send for HipDevice {}
unsafe impl Sync for HipDevice {}
unsafe impl Send for HipBufferPool {}
unsafe impl Sync for HipBufferPool {}
unsafe impl Send for HipAllocator {}
unsafe impl Sync for HipAllocator {}
unsafe impl Send for HipPoolAllocator {}
unsafe impl Sync for HipPoolAllocator {}
unsafe impl Send for HipStream {}
unsafe impl Sync for HipStream {}
pub use hip_stream::HipStream;
pub use hip_video_frame::{
    HipVideoFrame, HipVideoFrameExt, HipVideoFrameRef, IsHipVideoFrame, Readable, Writable,
};

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

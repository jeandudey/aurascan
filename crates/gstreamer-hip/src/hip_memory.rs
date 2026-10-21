use ffi::GstHipMemory;
use glib::translate::*;
use gst::{Memory, MemoryRef};

use crate::{ffi, HipStream};

gst::memory_object_wrapper! {
    HipMemory,
    HipMemoryRef,
    GstHipMemory,
    |mem: &MemoryRef| unsafe { from_glib(ffi::gst_is_hip_memory(mem.as_mut_ptr())) },
    Memory,
    MemoryRef,
}

impl std::fmt::Debug for HipMemory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        HipMemoryRef::fmt(self, f)
    }
}

impl std::fmt::Debug for HipMemoryRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        gst::MemoryRef::fmt(self, f)
    }
}

impl HipMemoryRef {
    #[doc(alias = "gst_hip_memory_get_stream")]
    pub fn stream(&self) -> Option<HipStream> {
        unsafe { from_glib_none(ffi::gst_hip_memory_get_stream(mut_override(&self.0))) }
    }

    pub fn video_info(&self) -> gst_video::VideoInfo {
        unsafe { from_glib_none(&(*self.as_ptr()).info as *const _) }
    }

    #[doc(alias = "gst_hip_memory_sync")]
    pub fn sync(&self) {
        unsafe { ffi::gst_hip_memory_sync(mut_override(&self.0)) }
    }
}

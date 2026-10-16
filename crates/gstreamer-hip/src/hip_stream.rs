use crate::ffi;

glib::wrapper! {
    #[doc(alias = "GstHipStream")]
    pub struct HipStream(Shared<ffi::GstHipStream>);

    match fn {
        ref => |ptr| ffi::gst_hip_stream_ref(ptr),
        unref => |ptr| ffi::gst_hip_stream_unref(ptr),
        type_ => || ffi::gst_hip_stream_get_type(),
    }
}

impl HipStream {
    #[doc(alias = "gst_hip_stream_get_device_id")]
    #[doc(alias = "get_device_id")]
    pub fn device_id(&self) -> u32 {
        unsafe { ffi::gst_hip_stream_get_device_id(self.to_glib_none().0) }
    }

    #[doc(alias = "gst_hip_stream_get_vendor")]
    #[doc(alias = "get_vendor")]
    pub fn vendor(&self) -> crate::HipVendor {
        unsafe { glib::translate::from_glib(ffi::gst_hip_stream_get_vendor(self.to_glib_none().0)) }
    }
}

use glib::translate::*;

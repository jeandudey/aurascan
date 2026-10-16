use crate::{ffi, HipDevice, HipVendor};
use glib::translate::*;
use gst::prelude::*;

/// Returns `true` if `mem` is a `GstHipMemory`.
#[doc(alias = "gst_is_hip_memory")]
pub fn is_hip_memory(mem: &gst::MemoryRef) -> bool {
    unsafe { from_glib(ffi::gst_is_hip_memory(mem.as_mut_ptr())) }
}

/// Waits until the work queued on the stream of a `GstHipMemory` is done.
#[doc(alias = "gst_hip_memory_sync")]
pub fn hip_memory_sync(mem: &gst::MemoryRef) {
    assert!(is_hip_memory(mem));
    unsafe { ffi::gst_hip_memory_sync(mem.as_mut_ptr() as *mut ffi::GstHipMemory) }
}

/// Returns the stream associated to a `GstHipMemory`.
#[doc(alias = "gst_hip_memory_get_stream")]
pub fn hip_memory_stream(mem: &gst::MemoryRef) -> Option<crate::HipStream> {
    assert!(is_hip_memory(mem));
    unsafe {
        from_glib_none(ffi::gst_hip_memory_get_stream(
            mem.as_mut_ptr() as *mut ffi::GstHipMemory
        ))
    }
}

/// Returns the device of a `GstHipMemory`.
pub fn hip_memory_device(mem: &gst::MemoryRef) -> HipDevice {
    assert!(is_hip_memory(mem));
    unsafe {
        from_glib_none((*(mem.as_mut_ptr() as *mut ffi::GstHipMemory)).device)
    }
}

/// Makes sure `device` is set on `element`, creating one if needed.
#[doc(alias = "gst_hip_ensure_element_data")]
pub fn hip_ensure_element_data(
    element: &impl IsA<gst::Element>,
    vendor: HipVendor,
    device_id: i32,
    device: &mut Option<HipDevice>,
) -> bool {
    unsafe {
        let mut ptr: *mut ffi::GstHipDevice = device.take().map_or(std::ptr::null_mut(), |d| d.into_glib_ptr());
        let ret = from_glib(ffi::gst_hip_ensure_element_data(
            element.as_ref().to_glib_none().0,
            vendor.into_glib(),
            device_id,
            &mut ptr,
        ));
        *device = from_glib_full(ptr);
        ret
    }
}

/// Handles a `GstContext` set on `element`.
#[doc(alias = "gst_hip_handle_set_context")]
pub fn hip_handle_set_context(
    element: &impl IsA<gst::Element>,
    context: &gst::Context,
    vendor: HipVendor,
    device_id: i32,
    device: &mut Option<HipDevice>,
) -> bool {
    unsafe {
        let mut ptr: *mut ffi::GstHipDevice = device.take().map_or(std::ptr::null_mut(), |d| d.into_glib_ptr());
        let ret = from_glib(ffi::gst_hip_handle_set_context(
            element.as_ref().to_glib_none().0,
            context.to_glib_none().0,
            vendor.into_glib(),
            device_id,
            &mut ptr,
        ));
        *device = from_glib_full(ptr);
        ret
    }
}

/// Answers a context query with `device`.
#[doc(alias = "gst_hip_handle_context_query")]
pub fn hip_handle_context_query(
    element: &impl IsA<gst::Element>,
    query: &mut gst::QueryRef,
    device: Option<&HipDevice>,
) -> bool {
    unsafe {
        from_glib(ffi::gst_hip_handle_context_query(
            element.as_ref().to_glib_none().0,
            query.as_mut_ptr(),
            device.to_glib_none().0,
        ))
    }
}

/// Creates a `GstContext` holding `device`.
#[doc(alias = "gst_context_new_hip_device")]
pub fn context_new_hip_device(device: &HipDevice) -> gst::Context {
    unsafe { from_glib_full(ffi::gst_context_new_hip_device(device.to_glib_none().0)) }
}

/// Returns the [`gst_video::VideoInfo`] describing the planes of a `GstHipMemory`
/// (offsets and strides).
pub fn hip_memory_video_info(mem: &gst::MemoryRef) -> gst_video::VideoInfo {
    assert!(is_hip_memory(mem));
    unsafe {
        let hip_mem = mem.as_mut_ptr() as *mut ffi::GstHipMemory;
        from_glib_none(&(*hip_mem).info as *const _)
    }
}

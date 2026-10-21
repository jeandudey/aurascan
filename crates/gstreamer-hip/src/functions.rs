use std::ptr;

use glib::{object::IsA, translate::*};

use crate::{ffi, HipDevice, HipVendor};

#[doc(alias = "gst_hip_handle_context_query")]
pub fn hip_handle_context_query(
    element: &impl IsA<gst::Element>,
    query: &mut gst::query::Context,
    device: Option<&impl IsA<HipDevice>>,
) -> bool {
    skip_assert_initialized!();
    unsafe {
        from_glib(ffi::gst_hip_handle_context_query(
            element.as_ref().to_glib_none().0,
            query.as_mut_ptr(),
            device.map(|p| p.as_ref()).to_glib_none().0,
        ))
    }
}

#[doc(alias = "gst_hip_handle_set_context")]
pub fn hip_handle_set_context(
    element: &impl IsA<gst::Element>,
    context: &gst::Context,
    vendor: HipVendor,
    device_id: i32,
) -> Option<HipDevice> {
    skip_assert_initialized!();
    unsafe {
        let mut device = ptr::null_mut();
        let ret = from_glib(ffi::gst_hip_handle_set_context(
            element.as_ref().to_glib_none().0,
            context.to_glib_none().0,
            vendor.into_glib(),
            device_id,
            &mut device,
        ));
        if ret {
            from_glib_full(device)
        } else {
            None
        }
    }
}

#[doc(alias = "gst_hip_ensure_element_data")]
pub fn hip_ensure_element_data(
    element: &impl IsA<gst::Element>,
    vendor: HipVendor,
    device_id: i32,
) -> Option<HipDevice> {
    skip_assert_initialized!();
    unsafe {
        let mut device = ptr::null_mut();
        let ret = from_glib(ffi::gst_hip_ensure_element_data(
            element.as_ref().to_glib_none().0,
            vendor.into_glib(),
            device_id,
            &mut device,
        ));
        if ret {
            from_glib_full(device)
        } else {
            None
        }
    }
}

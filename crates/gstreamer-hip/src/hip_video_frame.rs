// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

use std::{fmt::Debug, marker::PhantomData, mem, ptr};

use crate::{ffi, HipMemoryRef};
use glib::translate::*;
use gst_video::{video_frame::IsVideoFrame, VideoFrameExt};

pub enum Readable {}
pub enum Writable {}

pub unsafe trait IsHipVideoFrame: IsVideoFrame {}

pub trait HipVideoFrameExt: IsHipVideoFrame + VideoFrameExt {
    #[inline]
    fn memory(&self, idx: u32) -> Result<&HipMemoryRef, glib::BoolError> {
        if idx >= self.info().n_planes() {
            return Err(glib::bool_error!(
                "Memory index higher than number of memories"
            ));
        }

        // All the planes live in a single memory unless the buffer has one memory per plane.
        let buffer = self.buffer();
        let idx = if buffer.n_memory() == 1 { 0 } else { idx };
        if idx as usize >= buffer.n_memory() {
            return Err(glib::bool_error!(
                "Memory index higher than number of memories"
            ));
        }

        unsafe {
            let ptr = gst::ffi::gst_buffer_peek_memory(buffer.as_mut_ptr(), idx);
            if ffi::gst_is_hip_memory(ptr) == glib::ffi::GTRUE {
                Ok(HipMemoryRef::from_ptr(ptr as _))
            } else {
                Err(glib::bool_error!("Memory is not a HipMemory"))
            }
        }
    }

    #[inline]
    fn plane_device_ptr(&self, plane: u32) -> Result<*const u8, glib::BoolError> {
        if plane >= self.info().n_planes() {
            return Err(glib::bool_error!(
                "Plane index higher than number of planes"
            ));
        }

        Ok(self.as_raw().data[plane as usize] as *const u8)
    }
}

impl<O: IsHipVideoFrame> HipVideoFrameExt for O {}

pub struct HipVideoFrame<T> {
    frame: gst_video::ffi::GstVideoFrame,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for HipVideoFrame<T> {}
unsafe impl<T> Sync for HipVideoFrame<T> {}

unsafe impl<T> IsVideoFrame for HipVideoFrame<T> {
    #[inline]
    fn as_raw(&self) -> &gst_video::ffi::GstVideoFrame {
        &self.frame
    }
}

unsafe impl<T> IsHipVideoFrame for HipVideoFrame<T> {}

impl<T> Debug for HipVideoFrame<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("HipVideoFrame")
            .field("flags", &self.flags())
            .field("id", &self.id())
            .field("buffer", &self.buffer())
            .field("info", &self.info())
            .finish()
    }
}

impl<T> HipVideoFrame<T> {
    #[inline]
    pub fn into_buffer(self) -> gst::Buffer {
        unsafe {
            let mut s = mem::ManuallyDrop::new(self);
            let buffer = from_glib_none(s.frame.buffer);
            gst_video::ffi::gst_video_frame_unmap(&mut s.frame);
            buffer
        }
    }

    #[inline]
    pub unsafe fn from_glib_full(frame: gst_video::ffi::GstVideoFrame) -> Self {
        Self {
            frame,
            phantom: PhantomData,
        }
    }

    #[inline]
    pub fn into_raw(self) -> gst_video::ffi::GstVideoFrame {
        let s = mem::ManuallyDrop::new(self);
        s.frame
    }

    #[inline]
    pub fn as_video_frame_hip_ref(&self) -> HipVideoFrameRef<&gst::BufferRef> {
        let frame = unsafe { ptr::read(&self.frame) };
        HipVideoFrameRef {
            frame,
            unmap: false,
            phantom: PhantomData,
        }
    }
}

impl<T> Drop for HipVideoFrame<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            gst_video::ffi::gst_video_frame_unmap(&mut self.frame);
        }
    }
}

impl HipVideoFrame<Readable> {
    #[inline]
    pub fn from_buffer_readable(
        buffer: gst::Buffer,
        info: &gst_video::VideoInfo,
    ) -> Result<Self, gst::Buffer> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer.as_ref()) {
            return Err(buffer);
        }

        unsafe {
            let mut frame = mem::MaybeUninit::uninit();
            let res: bool = from_glib(gst_video::ffi::gst_video_frame_map(
                frame.as_mut_ptr(),
                info.to_glib_none().0 as *mut _,
                buffer.to_glib_none().0,
                gst::ffi::GST_MAP_READ | ffi::GST_MAP_HIP,
            ));

            if !res {
                Err(buffer)
            } else {
                Ok(Self {
                    frame: frame.assume_init(),
                    phantom: PhantomData,
                })
            }
        }
    }
}

impl HipVideoFrame<Writable> {
    #[inline]
    pub fn from_buffer_writable(
        buffer: gst::Buffer,
        info: &gst_video::VideoInfo,
    ) -> Result<Self, gst::Buffer> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer.as_ref()) {
            return Err(buffer);
        }

        unsafe {
            let mut frame = mem::MaybeUninit::uninit();
            let res: bool = from_glib(gst_video::ffi::gst_video_frame_map(
                frame.as_mut_ptr(),
                info.to_glib_none().0 as *mut _,
                buffer.to_glib_none().0,
                gst::ffi::GST_MAP_READ | gst::ffi::GST_MAP_WRITE | ffi::GST_MAP_HIP,
            ));

            if !res {
                Err(buffer)
            } else {
                Ok(Self {
                    frame: frame.assume_init(),
                    phantom: PhantomData,
                })
            }
        }
    }

    #[inline]
    pub fn plane_device_ptr_mut(&mut self, plane: u32) -> Result<*mut u8, glib::BoolError> {
        Ok(self.plane_device_ptr(plane)? as *mut u8)
    }

    #[inline]
    pub fn buffer_mut(&mut self) -> &mut gst::BufferRef {
        unsafe { gst::BufferRef::from_mut_ptr(self.frame.buffer) }
    }
}

pub struct HipVideoFrameRef<T> {
    frame: gst_video::ffi::GstVideoFrame,
    unmap: bool,
    phantom: PhantomData<T>,
}

unsafe impl<T> Send for HipVideoFrameRef<T> {}
unsafe impl<T> Sync for HipVideoFrameRef<T> {}

unsafe impl<T> IsVideoFrame for HipVideoFrameRef<T> {
    #[inline]
    fn as_raw(&self) -> &gst_video::ffi::GstVideoFrame {
        &self.frame
    }
}

unsafe impl<T> IsHipVideoFrame for HipVideoFrameRef<T> {}

impl<T> Debug for HipVideoFrameRef<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("HipVideoFrameRef")
            .field("flags", &self.flags())
            .field("id", &self.id())
            .field("buffer", &unsafe {
                gst::BufferRef::from_ptr(self.frame.buffer)
            })
            .field("info", &self.info())
            .finish()
    }
}

impl<'a> HipVideoFrameRef<&'a gst::BufferRef> {
    #[inline]
    pub fn from_buffer_ref_readable<'b>(
        buffer: &'a gst::BufferRef,
        info: &'b gst_video::VideoInfo,
    ) -> Result<HipVideoFrameRef<&'a gst::BufferRef>, glib::error::BoolError> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer) {
            return Err(glib::bool_error!("Memory is not a GstHipMemory"));
        }

        unsafe {
            let mut frame = mem::MaybeUninit::uninit();
            let res: bool = from_glib(gst_video::ffi::gst_video_frame_map(
                frame.as_mut_ptr(),
                info.to_glib_none().0 as *mut _,
                buffer.as_mut_ptr(),
                gst_video::ffi::GST_VIDEO_FRAME_MAP_FLAG_NO_REF
                    | gst::ffi::GST_MAP_READ
                    | ffi::GST_MAP_HIP,
            ));

            if !res {
                Err(glib::bool_error!(
                    "Failed to fill in the values of GstVideoFrame"
                ))
            } else {
                Ok(Self {
                    frame: frame.assume_init(),
                    unmap: true,
                    phantom: PhantomData,
                })
            }
        }
    }
}

impl<'a> HipVideoFrameRef<&'a mut gst::BufferRef> {
    #[inline]
    pub fn from_buffer_ref_writable<'b>(
        buffer: &'a mut gst::BufferRef,
        info: &'b gst_video::VideoInfo,
    ) -> Result<HipVideoFrameRef<&'a mut gst::BufferRef>, glib::error::BoolError> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer) {
            return Err(glib::bool_error!("Memory is not a GstHipMemory"));
        }

        unsafe {
            let mut frame = mem::MaybeUninit::uninit();
            let res: bool = from_glib(gst_video::ffi::gst_video_frame_map(
                frame.as_mut_ptr(),
                info.to_glib_none().0 as *mut _,
                buffer.as_mut_ptr(),
                gst_video::ffi::GST_VIDEO_FRAME_MAP_FLAG_NO_REF
                    | gst::ffi::GST_MAP_READ
                    | gst::ffi::GST_MAP_WRITE
                    | ffi::GST_MAP_HIP,
            ));

            if !res {
                Err(glib::bool_error!(
                    "Failed to fill in the values of GstVideoFrame"
                ))
            } else {
                Ok(Self {
                    frame: frame.assume_init(),
                    unmap: true,
                    phantom: PhantomData,
                })
            }
        }
    }

    #[inline]
    pub fn plane_device_ptr_mut(&mut self, plane: u32) -> Result<*mut u8, glib::BoolError> {
        Ok(self.plane_device_ptr(plane)? as *mut u8)
    }

    #[inline]
    pub fn buffer_mut(&mut self) -> &mut gst::BufferRef {
        unsafe { gst::BufferRef::from_mut_ptr(self.frame.buffer) }
    }
}

impl<'a> std::ops::Deref for HipVideoFrameRef<&'a mut gst::BufferRef> {
    type Target = HipVideoFrameRef<&'a gst::BufferRef>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        unsafe { &*(self as *const Self as *const Self::Target) }
    }
}

impl<T> Drop for HipVideoFrameRef<T> {
    #[inline]
    fn drop(&mut self) {
        unsafe {
            if self.unmap {
                gst_video::ffi::gst_video_frame_unmap(&mut self.frame);
            }
        }
    }
}

fn buffer_is_hip(buffer: &gst::BufferRef) -> bool {
    skip_assert_initialized!();
    buffer.n_memory() > 0
        && buffer
            .iter_memories()
            .all(|mem| unsafe { ffi::gst_is_hip_memory(mem.as_mut_ptr()) == glib::ffi::GTRUE })
}

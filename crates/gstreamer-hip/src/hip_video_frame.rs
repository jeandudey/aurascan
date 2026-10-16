// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

//! Video frames of `GstHipMemory` mapped as device memory, modelled after `GLVideoFrame` of
//! `gstreamer-gl`.
//!
//! The plane pointers of a mapped frame are **HIP device pointers**: they can be passed to HIP
//! calls but must not be dereferenced on the CPU.

use std::{fmt::Debug, marker::PhantomData, mem};

use glib::translate::*;
use gst_video::{video_frame::IsVideoFrame, VideoFrameExt};

use crate::{ffi, is_hip_memory};

pub enum Readable {}
pub enum Writable {}

/// A mapped video frame whose memories are `GstHipMemory`.
///
/// # Safety
///
/// The frame must have been mapped with `GST_MAP_HIP`, so that its plane pointers are device
/// pointers, and all its memories must be `GstHipMemory`.
pub unsafe trait IsHipVideoFrame: IsVideoFrame {}

pub trait HipVideoFrameExt: IsHipVideoFrame + VideoFrameExt {
    /// The HIP memory backing `plane`.
    #[inline]
    fn memory(&self, plane: u32) -> Result<&gst::MemoryRef, glib::BoolError> {
        if plane >= self.info().n_planes() {
            return Err(glib::bool_error!(
                "Plane index higher than number of planes"
            ));
        }

        unsafe {
            let mem = gst::MemoryRef::from_ptr(self.as_raw().map[plane as usize].memory);
            if is_hip_memory(mem) {
                Ok(mem)
            } else {
                Err(glib::bool_error!("Memory is not a GstHipMemory"))
            }
        }
    }

    /// The device pointer to the first byte of `plane`.
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

/// Whether all memories of `buffer` are `GstHipMemory`.
fn buffer_is_hip(buffer: &gst::BufferRef) -> bool {
    skip_assert_initialized!();
    buffer.n_memory() > 0 && buffer.iter_memories().all(is_hip_memory)
}

/// Maps `buffer` with the HIP flag and `flags`.
fn map_frame(
    buffer: *mut gst::ffi::GstBuffer,
    info: &gst_video::VideoInfo,
    flags: u32,
) -> Result<gst_video::ffi::GstVideoFrame, glib::BoolError> {
    skip_assert_initialized!();
    unsafe {
        let mut frame = mem::MaybeUninit::uninit();
        let res: bool = from_glib(gst_video::ffi::gst_video_frame_map(
            frame.as_mut_ptr(),
            info.to_glib_none().0 as *mut _,
            buffer,
            flags | ffi::GST_MAP_HIP,
        ));

        if res {
            Ok(frame.assume_init())
        } else {
            Err(glib::bool_error!(
                "Failed to fill in the values of GstVideoFrame"
            ))
        }
    }
}

/// An owned video frame of `GstHipMemory`, unmapped when dropped.
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

        match map_frame(buffer.to_glib_none().0, info, gst::ffi::GST_MAP_READ) {
            Ok(frame) => Ok(Self {
                frame,
                phantom: PhantomData,
            }),
            Err(_) => Err(buffer),
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

        match map_frame(
            buffer.to_glib_none().0,
            info,
            gst::ffi::GST_MAP_READ | gst::ffi::GST_MAP_WRITE,
        ) {
            Ok(frame) => Ok(Self {
                frame,
                phantom: PhantomData,
            }),
            Err(_) => Err(buffer),
        }
    }

    /// The device pointer to the first byte of `plane`, for writing.
    #[inline]
    pub fn plane_device_ptr_mut(&mut self, plane: u32) -> Result<*mut u8, glib::BoolError> {
        Ok(self.plane_device_ptr(plane)? as *mut u8)
    }

    #[inline]
    pub fn buffer_mut(&mut self) -> &mut gst::BufferRef {
        unsafe { gst::BufferRef::from_mut_ptr(self.frame.buffer) }
    }
}

/// A video frame of `GstHipMemory` that borrows its buffer, unmapped when dropped.
pub struct HipVideoFrameRef<T> {
    frame: gst_video::ffi::GstVideoFrame,
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
    pub fn from_buffer_ref_readable(
        buffer: &'a gst::BufferRef,
        info: &gst_video::VideoInfo,
    ) -> Result<Self, glib::BoolError> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer) {
            return Err(glib::bool_error!("Memory is not a GstHipMemory"));
        }

        let frame = map_frame(
            buffer.as_mut_ptr(),
            info,
            gst_video::ffi::GST_VIDEO_FRAME_MAP_FLAG_NO_REF | gst::ffi::GST_MAP_READ,
        )?;
        Ok(Self {
            frame,
            phantom: PhantomData,
        })
    }
}

impl<'a> HipVideoFrameRef<&'a mut gst::BufferRef> {
    #[inline]
    pub fn from_buffer_ref_writable(
        buffer: &'a mut gst::BufferRef,
        info: &gst_video::VideoInfo,
    ) -> Result<Self, glib::BoolError> {
        skip_assert_initialized!();

        if !buffer_is_hip(buffer) {
            return Err(glib::bool_error!("Memory is not a GstHipMemory"));
        }

        let frame = map_frame(
            buffer.as_mut_ptr(),
            info,
            gst_video::ffi::GST_VIDEO_FRAME_MAP_FLAG_NO_REF
                | gst::ffi::GST_MAP_READ
                | gst::ffi::GST_MAP_WRITE,
        )?;
        Ok(Self {
            frame,
            phantom: PhantomData,
        })
    }

    /// The device pointer to the first byte of `plane`, for writing.
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
            gst_video::ffi::gst_video_frame_unmap(&mut self.frame);
        }
    }
}

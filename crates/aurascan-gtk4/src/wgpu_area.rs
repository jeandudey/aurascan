// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use gtk::glib::subclass::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};
use std::cell::Ref;

mod imp {
    use std::cell::RefCell;
    use std::ffi::{CString, c_void};
    use std::num::NonZeroU32;
    use std::ops::Deref;
    use std::sync::LazyLock;

    use glow::HasContext;
    use wgpu::hal::Adapter;

    use super::*;

    #[derive(Debug)]
    pub enum Attachment {
        Renderbuffer(glow::NativeRenderbuffer),
        Texture(glow::NativeTexture),
    }

    #[derive(Debug)]
    pub struct State {
        pub gl: glow::Context,
        pub instance: wgpu::Instance,
        pub adapter: wgpu::Adapter,
        pub device: wgpu::Device,
        pub queue: wgpu::Queue,
    }

    #[derive(Debug, Default)]
    pub struct WGPUArea {
        pub state: RefCell<Option<State>>,
        pub size: RefCell<(i32, i32)>,
        pub color_view: RefCell<Option<wgpu::TextureView>>,
        pub depth_view: RefCell<Option<wgpu::TextureView>>,
    }

    impl WGPUArea {
        pub fn state(&self) -> Ref<'_, State> {
            Ref::map(self.state.borrow(), |state| {
                state.as_ref().expect("not realized")
            })
        }

        pub fn size(&self) -> wgpu::Extent3d {
            let (width, height) = *self.size.borrow();

            wgpu::Extent3d {
                width: width as u32,
                height: height as u32,
                depth_or_array_layers: 1,
            }
        }

        pub unsafe fn as_hal(
            &self,
        ) -> impl Deref<Target = wgpu::hal::gles::Device> + wgpu::WasmNotSendSync {
            unsafe {
                self.state()
                    .device
                    .as_hal::<wgpu::hal::gles::Api>()
                    .expect("backend should be gles")
            }
        }

        pub unsafe fn query_attachment(&self, attachment: u32) -> Option<Attachment> {
            let kind = unsafe {
                self.state().gl.get_framebuffer_attachment_parameter_i32(
                    glow::DRAW_FRAMEBUFFER,
                    attachment,
                    glow::FRAMEBUFFER_ATTACHMENT_OBJECT_TYPE,
                )
            };

            let name = unsafe {
                self.state().gl.get_framebuffer_attachment_parameter_i32(
                    glow::DRAW_FRAMEBUFFER,
                    attachment,
                    glow::FRAMEBUFFER_ATTACHMENT_OBJECT_NAME,
                )
            };

            let name = NonZeroU32::new(name as u32)?;

            match kind {
                x if x == glow::TEXTURE as i32 => {
                    Some(Attachment::Texture(glow::NativeTexture(name)))
                }
                x if x == glow::RENDERBUFFER as i32 => {
                    Some(Attachment::Renderbuffer(glow::NativeRenderbuffer(name)))
                }
                _ => None,
            }
        }

        pub unsafe fn texture_from_raw(
            &self,
            texture: glow::NativeTexture,
            label: Option<&str>,
            format: wgpu::TextureFormat,
        ) -> wgpu::Texture {
            let size = self.size();

            let hal_texture = unsafe {
                self.as_hal().texture_from_raw(
                    texture.0,
                    &wgpu::hal::TextureDescriptor {
                        label,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUses::UNINITIALIZED,
                        memory_flags: wgpu::hal::MemoryFlags::empty(),
                        view_formats: Vec::new(),
                    },
                    // Don't drop, owned by GTK.
                    Some(Box::new(|| {})),
                )
            };

            unsafe {
                self.state()
                    .device
                    .create_texture_from_hal::<wgpu::hal::gles::Api>(
                        hal_texture,
                        &wgpu::TextureDescriptor {
                            label,
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                    )
            }
        }

        pub unsafe fn texture_from_raw_renderbuffer(
            &self,
            renderbuffer: glow::NativeRenderbuffer,
            label: Option<&str>,
            format: wgpu::TextureFormat,
        ) -> wgpu::Texture {
            let size = self.size();

            let hal_texture = unsafe {
                self.as_hal().texture_from_raw_renderbuffer(
                    renderbuffer.0,
                    &wgpu::hal::TextureDescriptor {
                        label,
                        size,
                        mip_level_count: 1,
                        sample_count: 1,
                        dimension: wgpu::TextureDimension::D2,
                        format,
                        usage: wgpu::TextureUses::UNINITIALIZED,
                        memory_flags: wgpu::hal::MemoryFlags::empty(),
                        view_formats: Vec::new(),
                    },
                    Some(Box::new(|| {})),
                )
            };

            unsafe {
                self.state()
                    .device
                    .create_texture_from_hal::<wgpu::hal::gles::Api>(
                        hal_texture,
                        &wgpu::wgt::TextureDescriptor {
                            label,
                            size,
                            mip_level_count: 1,
                            sample_count: 1,
                            dimension: wgpu::TextureDimension::D2,
                            format,
                            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                            view_formats: &[],
                        },
                    )
            }
        }

        pub unsafe fn texture_from_attachment(
            &self,
            attachment: Attachment,
            label: Option<&str>,
            format: wgpu::TextureFormat,
        ) -> wgpu::Texture {
            match attachment {
                Attachment::Texture(texture) => unsafe {
                    self.texture_from_raw(texture, label, format)
                },
                Attachment::Renderbuffer(renderbuffer) => unsafe {
                    self.texture_from_raw_renderbuffer(renderbuffer, label, format)
                },
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for WGPUArea {
        const NAME: &'static str = "AscWGPUArea";
        type Type = super::WGPUArea;
        type ParentType = gtk::GLArea;
    }

    impl ObjectImpl for WGPUArea {
        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: LazyLock<Vec<Signal>> =
                LazyLock::new(|| vec![Signal::builder("render-wgpu").build()]);

            &SIGNALS
        }

        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            obj.set_use_es(true);
            obj.set_has_stencil_buffer(true);
            obj.set_has_depth_buffer(true);
        }
    }

    impl WidgetImpl for WGPUArea {
        fn realize(&self) {
            self.parent_realize();

            let obj = self.obj();
            obj.make_current();

            // SAFETY:
            // - The context is current when calling `obj.make_current()`.
            // - The context is always current when interacting with the
            // adapter (realize, unrealize and render).
            // - The context is current when dropping the adapter and any
            // objects created with it.
            let exposed_adapter = unsafe { epoxy_wgpu_adapter() };

            let hal_device = unsafe {
                match exposed_adapter.adapter.open(
                    wgpu::Features::empty(),
                    &wgpu::Limits::default(),
                    &wgpu::MemoryHints::Performance,
                ) {
                    Ok(v) => v,
                    Err(err) => {
                        glib::g_error!("AscWGPUArea", "Failed to open adapter: {}", err);
                        return;
                    }
                }
            };

            let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
                backends: wgpu::Backends::GL,
                flags: wgpu::InstanceFlags::default(),
                memory_budget_thresholds: wgpu::MemoryBudgetThresholds::default(),
                backend_options: wgpu::BackendOptions::default(),
                // Not needed, gtk::GLArea provides the GLES context and the FBO.
                display: None,
            });

            let adapter = unsafe { instance.create_adapter_from_hal(exposed_adapter) };

            let (device, queue) = unsafe {
                match adapter.create_device_from_hal(
                    hal_device,
                    &wgpu::DeviceDescriptor {
                        label: Some("wgpuarea-device"),
                        ..Default::default()
                    },
                ) {
                    Ok(v) => v,
                    Err(err) => {
                        glib::g_error!("AscWGPUArea", "Failed to create device from HAL: {}", err);
                        return;
                    }
                }
            };

            let gl = unsafe { glow::Context::from_loader_function(epoxy_egl_get_proc_address) };

            *self.state.borrow_mut() = Some(State {
                gl,
                instance,
                adapter,
                device,
                queue,
            });
        }

        fn unrealize(&self) {
            self.obj().make_current();

            // Drop wgpu resources before dropping the GL context.
            *self.state.borrow_mut() = None;

            self.parent_unrealize();
        }
    }

    impl GLAreaImpl for WGPUArea {
        fn resize(&self, width: i32, height: i32) {
            *self.size.borrow_mut() = (width, height);
        }

        fn render(&self, _context: &gdk::GLContext) -> glib::Propagation {
            self.obj().make_current();

            *self.color_view.borrow_mut() = None;
            *self.depth_view.borrow_mut() = None;

            let color_format = wgpu::TextureFormat::Rgba8Unorm;
            let color_texture = unsafe {
                self.query_attachment(glow::COLOR_ATTACHMENT0)
                    .map(|attachment| {
                        self.texture_from_attachment(
                            attachment,
                            Some("wgpuarea-color-attachment"),
                            color_format,
                        )
                    })
            };
            let Some(color_texture) = color_texture else {
                glib::g_error!("AscWGPUArea", "Failed to query color attachment");
                return glib::Propagation::Stop;
            };

            let depth_format = wgpu::TextureFormat::Depth24PlusStencil8;
            let depth_texture = unsafe {
                self.query_attachment(glow::DEPTH_STENCIL_ATTACHMENT)
                    .or_else(|| self.query_attachment(glow::DEPTH_ATTACHMENT))
                    .map(|attachment| {
                        self.texture_from_attachment(
                            attachment,
                            Some("wgpuarea-depth"),
                            depth_format,
                        )
                    })
            };
            let Some(depth_texture) = depth_texture else {
                glib::g_error!("AscWGPUArea", "Failed to query depth attachment");
                return glib::Propagation::Stop;
            };

            let color_view = color_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("wgpuarea-color-view"),
                ..Default::default()
            });
            let depth_view = depth_texture.create_view(&wgpu::TextureViewDescriptor {
                label: Some("wgpuare-depth-view"),
                ..Default::default()
            });

            *self.color_view.borrow_mut() = Some(color_view);
            *self.depth_view.borrow_mut() = Some(depth_view);

            self.obj().emit_by_name::<()>("render-wgpu", &[]);

            self.state()
                .device
                .poll(wgpu::PollType::Wait {
                    submission_index: None,
                    timeout: None,
                })
                .expect("the submission index is not specified and there is no timeout");

            *self.color_view.borrow_mut() = None;
            *self.depth_view.borrow_mut() = None;

            self.obj().attach_buffers();

            glib::Propagation::Proceed
        }
    }

    /// Create a GLES exposed adapter for [`wgpu`].
    ///
    /// # Safety
    ///
    /// Same rules as as [`wgpu::hal::gles::Adapter::new_external`].
    unsafe fn epoxy_wgpu_adapter() -> wgpu::hal::ExposedAdapter<wgpu::hal::gles::Api> {
        let options = wgpu::GlBackendOptions::default();
        let result =
            unsafe { wgpu::hal::gles::Adapter::new_external(epoxy_egl_get_proc_address, options) };
        result.unwrap()
    }

    fn epoxy_egl_get_proc_address(name: &str) -> *const c_void {
        let name = CString::new(name).expect("procedure name shouldn't have NUL bytes");
        let epoxy_egl_get_proc_address = unsafe { epoxy_sys::epoxy_eglGetProcAddress.unwrap() };
        unsafe {
            epoxy_egl_get_proc_address(name.as_ptr())
                .map(|f| f as *const c_void)
                .expect("Failed to get proc address")
        }
    }
}

glib::wrapper! {
    pub struct WGPUArea(ObjectSubclass<imp::WGPUArea>)
        @extends gtk::Widget, gtk::GLArea,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Default for WGPUArea {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl WGPUArea {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn instance(&self) -> Option<Ref<'_, wgpu::Instance>> {
        Ref::filter_map(self.imp().state.borrow(), |v| {
            v.as_ref().map(|s| &s.instance)
        })
        .ok()
    }

    pub fn adapter(&self) -> Option<Ref<'_, wgpu::Adapter>> {
        Ref::filter_map(self.imp().state.borrow(), |v| {
            v.as_ref().map(|s| &s.adapter)
        })
        .ok()
    }

    pub fn device(&self) -> Option<Ref<'_, wgpu::Device>> {
        Ref::filter_map(self.imp().state.borrow(), |v| v.as_ref().map(|s| &s.device)).ok()
    }

    pub fn queue(&self) -> Option<Ref<'_, wgpu::Queue>> {
        Ref::filter_map(self.imp().state.borrow(), |v| v.as_ref().map(|s| &s.queue)).ok()
    }

    pub fn color_view(&self) -> Option<Ref<'_, wgpu::TextureView>> {
        Ref::filter_map(self.imp().color_view.borrow(), |v| v.as_ref()).ok()
    }

    pub fn depth_view(&self) -> Option<Ref<'_, wgpu::TextureView>> {
        Ref::filter_map(self.imp().depth_view.borrow(), |v| v.as_ref()).ok()
    }

    pub fn connect_render_wgpu<F>(&self, f: F) -> glib::SignalHandlerId
    where
        F: Fn(WGPUArea) + 'static,
    {
        self.connect_closure(
            "render-wgpu",
            false,
            glib::closure_local!(move |wgpu_area: WGPUArea| { f(wgpu_area) }),
        )
    }
}

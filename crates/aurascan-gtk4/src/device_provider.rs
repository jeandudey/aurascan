// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use std::collections::HashSet;
use std::sync::Once;

use gst::prelude::*;
use gtk::gio::prelude::ListModelExt;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use thiserror::Error;

const PROVIDER_NAME: &str = "pipewiredeviceprovider";

static STARTED: Once = Once::new();

mod imp {
    use std::cell::{OnceCell, RefCell};
    use std::sync::LazyLock;

    use glib::Properties;

    use super::*;

    #[derive(Default, Properties)]
    #[properties(wrapper_type = super::DeviceProvider)]
    pub struct DeviceProvider {
        pub inner: OnceCell<gst::DeviceProvider>,
        pub bus_watch: OnceCell<gst::bus::BusWatchGuard>,
        pub cameras: RefCell<Vec<crate::Camera>>,
        pub default_cb: OnceCell<Box<dyn Fn(&crate::Camera) -> bool + 'static>>,

        #[property(get = Self::started)]
        pub started: std::marker::PhantomData<bool>,
    }

    impl DeviceProvider {
        fn started(&self) -> bool {
            STARTED.is_completed()
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for DeviceProvider {
        const NAME: &'static str = "AscDeviceProvider";
        type Type = super::DeviceProvider;
        type Interfaces = (gio::ListModel,);
    }

    #[glib::derived_properties]
    impl ObjectImpl for DeviceProvider {
        fn constructed(&self) {
            self.parent_constructed();

            if let Some(provider) = gst::DeviceProviderFactory::by_name(PROVIDER_NAME) {
                self.inner.set(provider).unwrap();
            } else {
                log::error!("Could not create DeviceProviderFactory with name {PROVIDER_NAME}");
            }
        }

        fn dispose(&self) {
            if let Some(provider) = self.inner.get() {
                if provider.is_started() {
                    provider.stop();
                }

                provider.set_property("fd", -1);
            }
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: LazyLock<Vec<glib::subclass::Signal>> = LazyLock::new(|| {
                vec![
                    glib::subclass::Signal::builder("camera-added").build(),
                    glib::subclass::Signal::builder("camera-removed").build(),
                ]
            });

            &SIGNALS
        }
    }

    impl ListModelImpl for DeviceProvider {
        fn item_type(&self) -> glib::Type {
            crate::Camera::static_type()
        }

        fn n_items(&self) -> u32 {
            self.cameras.borrow().len() as u32
        }

        fn item(&self, position: u32) -> Option<glib::Object> {
            self.cameras
                .borrow()
                .get(position as usize)
                .cloned()
                .and_upcast()
        }
    }
}

glib::wrapper! {
    pub struct DeviceProvider(ObjectSubclass<imp::DeviceProvider>)
        @implements gio::ListModel;
}

impl DeviceProvider {
    pub fn instance() -> &'static Self {
        use std::sync::LazyLock;

        use glib::thread_guard::ThreadGuard;

        struct Wrapper(ThreadGuard<crate::DeviceProvider>);

        static SINGLETON: LazyLock<Wrapper> = LazyLock::new(|| {
            Wrapper(ThreadGuard::new(
                glib::Object::new::<crate::DeviceProvider>(),
            ))
        });

        SINGLETON.0.get_ref()
    }

    pub fn start_with_default<F>(&self, f: F) -> Result<(), DeviceProviderError>
    where
        F: Fn(&crate::Camera) -> bool + 'static,
    {
        if STARTED.is_completed() {
            return Ok(());
        }

        STARTED.call_once(|| {});

        let imp = self.imp();

        let provider = imp
            .inner
            .get()
            .ok_or(DeviceProviderError::MissingPlugin(PROVIDER_NAME))?;
        provider.start()?;

        let mut seen = HashSet::new();
        let mut cameras = provider
            .devices()
            .iter()
            .filter(|device| is_camera(device))
            .map(crate::Camera::new)
            .filter(|device| is_ir_camera(device))
            .collect::<Vec<_>>();
        cameras.retain(|item| seen.insert(item.target_object()));
        cameras.iter().for_each(|camera| {
            log::debug!(
                "Camera found: {}, target-object: {:?}\nProperties: {:#?}\nCaps: {:#?}",
                camera.display_name(),
                camera.target_object(),
                camera.properties(),
                camera.caps(),
            )
        });

        let n_items = cameras.len() as u32;
        imp.cameras.replace(cameras);
        self.items_changed(0, 0, n_items);

        let bus = provider.bus();
        let watch = bus
            .add_watch_local(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move |_, msg| { glib::ControlFlow::Continue }
            ))
            .expect("Failed to add bus watch");
        imp.bus_watch.set(watch).unwrap();

        let _ = imp.default_cb.set(Box::new(f));

        self.notify_started();

        Ok(())
    }

    pub fn start(&self) -> Result<(), DeviceProviderError> {
        self.start_with_default(|_| false)
    }

    pub(crate) fn default_camera(&self) -> Option<crate::Camera> {
        let imp = self.imp();
        let cameras = imp.cameras.borrow();
        imp.default_cb
            .get()
            .and_then(|f| cameras.iter().find(|camera| f(camera)))
            .cloned()
    }

    /// Gets a [`Camera`] object for the given camera index.
    ///
    /// # Returns
    ///
    /// a [`Camera`] at `position`.
    ///
    /// [`Camera`]: crate::Camera
    pub fn camera(&self, position: u32) -> Option<crate::Camera> {
        self.item(position).and_downcast()
    }
}

#[derive(Debug, Error)]
pub enum DeviceProviderError {
    #[error("missing gstreamer plugin: {0}")]
    MissingPlugin(&'static str),
    #[error("bool error: {0}")]
    BoolError(#[from] glib::BoolError),
}

fn is_camera(device: &gst::Device) -> bool {
    device.has_classes("Video/Source")
        && device
            .caps()
            .is_some_and(|c| c.can_intersect(&crate::SUPPORTED_CAPS))
}

fn is_ir_camera(device: &crate::Camera) -> bool {
    device.device().caps().as_ref().is_some_and(is_infrared)
        || device.nick().is_some_and(|nick| contains_ir(&nick))
        || contains_ir(&device.display_name())
}

fn is_infrared(caps: &gst::Caps) -> bool {
    caps.is_subset(&crate::IR_CAPS)
}

fn contains_ir(s: &str) -> bool {
    s.starts_with("IR ") || s.contains(" IR ") || s.ends_with(" IR")
}

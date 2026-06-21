// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: GPL-3.0-or-later

use gst::prelude::*;
use gtk::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gdk, glib};

#[derive(Default, Debug, Clone, Copy, PartialEq, glib::Enum)]
#[enum_type(name = "AscViewfinderState")]
pub enum ViewfinderState {
    #[default]
    Loading,
    Ready,
    NoCameras,
    Error,
}

#[derive(Debug)]
enum StateChangeState {
    Equal,
    Differ,
    Error,
    NotDone,
}

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};
    use std::sync::LazyLock;

    use glib::Properties;

    use super::*;

    #[derive(Debug, Default, Properties)]
    #[properties(wrapper_type = super::Viewfinder)]
    pub struct Viewfinder {
        #[property(get, explicit_notify, default)]
        state: Cell<ViewfinderState>,
        #[property(get, set = Self::set_camera, nullable, explicit_notify)]
        camera: RefCell<Option<crate::Camera>>,
        #[property(get, set, construct_only)]
        application_id: RefCell<glib::GString>,

        pub camerabin: OnceCell<gst::Element>,
        pub camera_element: OnceCell<gst::Element>,

        pub devices: OnceCell<crate::DeviceProvider>,
        pub bus_watch: OnceCell<gst::bus::BusWatchGuard>,
        pub tee: OnceCell<crate::PipelineTee>,

        picture: gtk::Picture,
        offload: gtk::GraphicsOffload,
    }

    impl Viewfinder {
        pub fn camerabin(&self) -> &gst::Element {
            self.camerabin.get().unwrap()
        }

        pub fn set_state(&self, state: ViewfinderState) {
            if state != self.state.replace(state) {
                self.obj().notify_state();
            }
        }

        fn set_camera(&self, camera: Option<crate::Camera>) {
            let obj = self.obj();

            if !matches!(obj.state(), ViewfinderState::Ready | ViewfinderState::Error) {
                log::error!("Could not set camera, the viewfinder is not ready");
                return;
            }

            if camera == self.camera.replace(camera.clone()) {
                return;
            }

            if matches!(obj.state(), ViewfinderState::Error) {
                if self
                    .devices
                    .get()
                    .and_then(|devices| devices.camera(0))
                    .is_some()
                {
                    self.set_state(ViewfinderState::Ready);
                } else {
                    self.set_state(ViewfinderState::NoCameras);
                }
            }

            if obj.is_realized()
                && matches!(
                    self.camerabin().current_state(),
                    gst::State::Playing | gst::State::Paused
                )
            {
                obj.stop_stream();
            }

            if let Some(camera) = camera
                && let Err(err) = obj.setup_camera_element(&camera)
            {
                log::error!("Could not reconfigure camera element: {err}");
                self.set_state(ViewfinderState::Error);
            }

            if obj.is_realized() && matches!(obj.state(), ViewfinderState::Ready) {
                obj.start_stream();
            }

            obj.notify_camera();
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Viewfinder {
        const NAME: &'static str = "AscViewfinder";
        type Type = super::Viewfinder;
        type ParentType = gtk::Widget;

        fn class_init(klass: &mut Self::Class) {
            klass.set_layout_manager_type::<gtk::BinLayout>();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Viewfinder {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let camerabin = gst::ElementFactory::make("camerabin")
                .property("location", None::<&str>)
                .build()
                .expect("Missing GStreamer Bad Plug-ins");
            self.camerabin.set(camerabin).unwrap();

            let bus = self.camerabin().bus().unwrap();
            let watch = bus
                .add_watch_local(glib::clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move |_, msg| {
                        obj.on_bus_message(msg);
                        glib::ControlFlow::Continue
                    }
                ))
                .unwrap();
            self.bus_watch.set(watch).unwrap();

            let tee = crate::PipelineTee::new();

            let paintablesink = gst::ElementFactory::make("gtk4paintablesink")
                .build()
                .expect("Missing gst-plugin-gtk4");

            let paintable = paintablesink.property::<gdk::Paintable>("paintable");

            let is_yuv_natively_supported = {
                let yuv_caps =
                    gst_video::video_make_raw_caps(&[gst_video::VideoFormat::Yuy2]).build();
                !paintablesink
                    .pad_template("sink")
                    .unwrap()
                    .caps()
                    .intersect(&yuv_caps)
                    .is_empty()
            };
            let sink = if is_yuv_natively_supported {
                let bin = gst::Bin::default();

                bin.add(&paintablesink).unwrap();
                bin.add_pad(
                    &gst::GhostPad::with_target(&paintablesink.static_pad("sink").unwrap())
                        .unwrap(),
                )
                .unwrap();

                bin.upcast()
            } else {
                let is_gl_supported = paintable
                    .property::<Option<gdk::GLContext>>("gl-context")
                    .is_some();
                if is_gl_supported {
                    gst::ElementFactory::make("glsinkbin")
                        .property("sink", &paintablesink)
                        .build()
                        .expect("Missing GStreamer Base Plug-ins")
                } else {
                    let bin = gst::Bin::default();
                    let convert = gst::ElementFactory::make("videoconvert")
                        .build()
                        .expect("Missing GStreamer Base Plug-ins");
                    bin.add(&convert).unwrap();
                    bin.add(&paintablesink).unwrap();
                    convert.link(&paintablesink).unwrap();

                    bin.add_pad(
                        &gst::GhostPad::with_target(&convert.static_pad("sink").unwrap()).unwrap(),
                    )
                    .unwrap();

                    bin.upcast()
                }
            };

            tee.add_branch(&sink);
            self.camerabin().set_property("viewfinder-sink", &tee);

            let caps_video = gst_video::video_make_raw_caps(&[
                gst_video::VideoFormat::I420,
                gst_video::VideoFormat::Nv12,
            ])
            .build();
            self.camerabin()
                .set_property("video-capture-caps", caps_video);

            self.picture
                .set_accessible_role(gtk::AccessibleRole::Presentation);
            self.picture.set_hexpand(true);
            self.picture.set_vexpand(true);
            self.picture.set_paintable(Some(&paintable));

            self.offload.set_child(Some(&self.picture));
            self.offload.set_parent(&*obj);
            self.offload.set_black_background(true);

            let devices = crate::DeviceProvider::instance();
            self.devices.set(devices.clone()).unwrap();

            if devices.started() {
                obj.init();
            } else {
                devices.connect_started_notify(glib::clone!(
                    #[weak]
                    obj,
                    move |_| {
                        obj.init();
                    }
                ));
            }
        }

        fn dispose(&self) {
            if let Err(err) = self.camerabin().set_state(gst::State::Null) {
                log::error!("Could not stop camerabin: {:?}", err);
            }

            self.offload.unparent();
        }

        fn signals() -> &'static [glib::subclass::Signal] {
            static SIGNALS: LazyLock<Vec<glib::subclass::Signal>> = LazyLock::new(|| {
                vec![
                    glib::subclass::Signal::builder("fps-update").build(),
                    glib::subclass::Signal::builder("head-tracking-sample").build(),
                ]
            });

            &SIGNALS
        }
    }

    impl WidgetImpl for Viewfinder {
        fn realize(&self) {
            self.parent_realize();

            log::debug!("Viewfinder state: {:?}", self.obj().state());

            if matches!(self.obj().state(), ViewfinderState::Ready) {
                log::debug!("Viewfinder realized: starting stream");
                self.obj().start_stream();
            }
        }

        fn unrealize(&self) {
            log::debug!("Viewfinder unrealized: stopping stream");
            self.obj().stop_stream();

            self.parent_unrealize();
        }
    }
}

glib::wrapper! {
    pub struct Viewfinder(ObjectSubclass<imp::Viewfinder>)
        @extends gtk::Widget,
        @implements gtk::Accessible, gtk::Buildable, gtk::ConstraintTarget;
}

impl Viewfinder {
    pub fn new(application_id: impl glib::IntoGStr) -> Self {
        application_id.run_with_gstr(|application_id| {
            glib::Object::builder()
                .property("application-id", application_id)
                .build()
        })
    }

    pub fn start_stream(&self) {
        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            async move {
                obj.change_status_inner(gst::State::Playing).await;
            }
        ));
    }

    async fn change_status_inner(&self, state: gst::State) {
        let (sender, receiver) = futures_channel::oneshot::channel();
        let camerabin = self.imp().camerabin();
        std::thread::spawn(glib::clone!(
            #[weak]
            camerabin,
            move || {
                let timeout = gst::format::ClockTime::from_seconds(2);
                let (res, current_state, pending_state) = camerabin.state(Some(timeout));
                let new_state_is = match res {
                    Ok(change_done) => {
                        if change_done == gst::StateChangeSuccess::Async {
                            camerabin.set_locked_state(true);
                            log::debug!(
                                "Camerabin could not change state from {current_state:?} to {pending_state:?}"
                            );
                            StateChangeState::NotDone
                        } else if current_state == state {
                            StateChangeState::Equal
                        } else {
                            StateChangeState::Differ
                        }
                    }
                    Err(err) => {
                        log::error!("Previous camerabin state change failed: {err}");
                        StateChangeState::Error
                    }
                };
                sender.send(new_state_is).unwrap();
            }
        )).join().unwrap();

        let change_state = receiver.await.unwrap();
        match change_state {
            StateChangeState::Equal => (),
            StateChangeState::NotDone => {
                log::debug!("Aborting camerabin state change {state:?}");
                camerabin.abort_state();
                camerabin.set_locked_state(false);
                self.set_camerabin_state(state);
            }
            StateChangeState::Error | StateChangeState::Differ => self.set_camerabin_state(state),
        }
    }

    fn set_camerabin_state(&self, state: gst::State) {
        match self.imp().camerabin().set_state(state) {
            Err(err) => {
                log::error!("Could not start camerabin: {err}");
                self.imp().set_state(ViewfinderState::Error);
            }
            Ok(gst::StateChangeSuccess::Async) => {
                log::debug!("Trying to set camerabin state to {state:?}");
            }
            Ok(_) => {
                log::debug!("Camerabin successfully state set to {state:?}");
            }
        }
    }

    pub fn stop_stream(&self) {
        if let Err(err) = self.imp().camerabin().set_state(gst::State::Null) {
            log::error!("Could not pause camerabin: {err}");
            self.imp().set_state(ViewfinderState::Error);
        } else {
            log::debug!("Camerabin state successfully set to NULL");
        }
    }

    fn create_camera_element(
        &self,
        device_src: &gst::Element,
    ) -> Result<gst::Element, glib::BoolError> {
        let bin = gst::Bin::new();

        let capsfilter = gst::ElementFactory::make("capsfilter").build()?;
        let decodebin3 = gst::ElementFactory::make("decodebin3").build()?;
        let capsfilter_post_decode = gst::ElementFactory::make("capsfilter").build()?;
        let caps_post_decode = gst::Caps::builder("video/x-raw").build();
        capsfilter_post_decode.set_property("caps", &caps_post_decode);

        bin.add_many(&[
            device_src,
            &capsfilter,
            &decodebin3,
            &capsfilter_post_decode,
        ])?;
        gst::Element::link_many([device_src, &capsfilter, &decodebin3])?;
        Ok(bin.upcast())
    }

    fn setup_camera_element(&self, camera: &crate::Camera) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        if let Some(element) = imp.camera_element.get() {
            camera.reconfigure(element)?;
        } else {
            let element = camera.create_element(&self.application_id())?;

            let wrapper = self.create_camera_element(&element)?;
            imp.camerabin().set_property("camera-source", &wrapper);

            imp.camera_element.set(element).unwrap();
        }

        Ok(())
    }

    fn init(&self) {
        log::debug!("Viewfinder init");

        let imp = self.imp();
        let devices = imp.devices.get().unwrap();

        if let Some(camera) = devices.default_camera().or_else(|| devices.camera(0))
            && matches!(
                self.state(),
                ViewfinderState::NoCameras | ViewfinderState::Loading | ViewfinderState::Error
            )
        {
            imp.set_state(ViewfinderState::Ready);
            self.set_camera(Some(camera));
        }

        glib::timeout_add_local_once(
            std::time::Duration::from_secs(2),
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move || {
                    if matches!(obj.state(), ViewfinderState::Loading) {
                        obj.imp().set_state(ViewfinderState::NoCameras);
                    }
                }
            ),
        );
    }

    fn on_bus_message(&self, msg: &gst::Message) {
        match msg.view() {
            _ => (),
        }
    }
}

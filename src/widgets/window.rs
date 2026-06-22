use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::widgets::PreferencesWindow;
use crate::{Application, app_id};

mod imp {
    use std::cell::{OnceCell, RefCell};

    use adw::subclass::prelude::*;
    use gtk::CompositeTemplate;

    use super::*;

    #[derive(Debug, CompositeTemplate)]
    #[template(resource = "/tech/jeandudey/Aurascan/ui/window.ui")]
    pub struct Window {
        #[template_child]
        pub viewfinder: TemplateChild<aurascan_gtk4::Viewfinder>,
        #[template_child]
        pub headposeview: TemplateChild<aurascan_gtk4::HeadPoseView>,
        #[template_child]
        pub play_button: TemplateChild<gtk::Button>,

        pub provider: OnceCell<aurascan_gtk4::DeviceProvider>,
        pub settings: gio::Settings,

        pub is_active_handle: RefCell<Option<glib::SignalHandlerId>>,
    }

    impl Window {
        pub async fn start(&self) {
            let obj = self.obj();
            let provider = self.provider.get().unwrap();

            glib::spawn_future_local(glib::clone!(
                #[weak]
                obj,
                #[strong]
                provider,
                async move {
                    if let Err(err) = provider.start_with_default(glib::clone!(
                        #[weak]
                        obj,
                        #[upgrade_or]
                        false,
                        move |camera| {
                            let stored_id = obj.imp().settings.string("last-camera-id");
                            !stored_id.is_empty()
                        }
                    )) {
                        log::error!("Could not start the device provider: {err}");
                    } else {
                        log::debug!("Device provider started");
                    }
                }
            ));
        }
    }

    impl Default for Window {
        fn default() -> Self {
            Self {
                viewfinder: Default::default(),
                headposeview: Default::default(),
                play_button: Default::default(),

                provider: Default::default(),
                settings: gio::Settings::new(app_id()),

                is_active_handle: Default::default(),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Window {
        const NAME: &'static str = "AscWindow";
        type Type = super::Window;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            //klass.bind_template_callbacks();

            klass.install_action("win.preferences", None, |window, _, _| {
                window.show_preferences_window();
            });

            klass.install_action("win.play", None, |window, _, _| {
                window.toggle_detection();
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for Window {
        fn constructed(&self) {
            self.parent_constructed();

            let provider = aurascan_gtk4::DeviceProvider::instance();
            self.provider.set(provider.clone()).unwrap();

            self.viewfinder
                .connect_fps_measurements(|_, fps, droprate, avgfps| {
                    log::info!("FPS: {fps}, Drop Rate: {droprate}, AVG: {avgfps}");
                });

            self.settings.connect_changed(
                Some("backend"),
                glib::clone!(
                    #[weak(rename_to = viewfinder)]
                    self.viewfinder,
                    move |settings, _| {
                        if viewfinder.detect_head_pose() {
                            log::warn!("Cannot change backend while detecting head pose");
                            return;
                        }

                        let backend = match settings.enum_("backend") {
                            0 => gstaurascan::BackendType::Flex,
                            1 => gstaurascan::BackendType::Vulkan,
                            2 => gstaurascan::BackendType::Rocm,
                            _ => {
                                log::warn!("Invalid backend type setting");
                                return;
                            }
                        };

                        viewfinder.set_backend(backend);
                    }
                ),
            );

            let obj = self.obj();

            obj.load_window_size();

            obj.connect_is_active_notify(|obj| {
                if !obj.is_active() {
                    return;
                }

                if let Some(handle) = obj.imp().is_active_handle.take() {
                    obj.disconnect(handle);
                }

                glib::spawn_future_local(glib::clone!(
                    #[weak]
                    obj,
                    async move {
                        obj.imp().start().await;
                    }
                ));
            });
        }
    }

    impl WidgetImpl for Window {}

    impl WindowImpl for Window {
        // Save window state on delete event
        fn close_request(&self) -> glib::Propagation {
            let window = self.obj();

            if let Err(err) = window.save_window_size() {
                log::warn!("Failed to save window state, {err:?}");
            }

            // Pass close request on to the parent
            self.parent_close_request()
        }
    }

    impl ApplicationWindowImpl for Window {}
    impl AdwApplicationWindowImpl for Window {}
}

glib::wrapper! {
    pub struct Window(ObjectSubclass<imp::Window>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionMap, gio::ActionGroup, gtk::ConstraintTarget, gtk::Accessible, gtk::Buildable, gtk::ShortcutManager, gtk::Native, gtk::Root;
}

impl Window {
    pub fn new(app: &Application) -> Self {
        glib::Object::builder()
            .property("application", &app)
            .build()
    }

    fn load_window_size(&self) {
        let imp = self.imp();

        let width = imp.settings.int("window-width");
        let height = imp.settings.int("window-height");
        let is_maximized = imp.settings.boolean("is-maximized");

        self.set_default_size(width, height);

        if is_maximized {
            self.maximize();
        }
    }

    fn save_window_size(&self) -> Result<(), glib::BoolError> {
        let imp = self.imp();

        let (width, height) = self.default_size();

        imp.settings.set_int("window-width", width)?;
        imp.settings.set_int("window-height", height)?;

        imp.settings
            .set_boolean("is-maximized", self.is_maximized())?;

        Ok(())
    }

    fn show_preferences_window(&self) {
        if self.visible_dialog().is_some() {
            return;
        }

        let is_detecting_head_pose = self.imp().viewfinder.detect_head_pose();
        let preferences = PreferencesWindow::new(is_detecting_head_pose);
        preferences.present(Some(self));
    }

    fn toggle_detection(&self) {
        let should_play = !self.imp().viewfinder.detect_head_pose();
        self.imp().viewfinder.set_detect_head_pose(should_play);

        if should_play {
            self.imp()
                .play_button
                .set_icon_name("media-playback-stop-symbolic");
        } else {
            self.imp()
                .play_button
                .set_icon_name("media-playback-start-symbolic");
        }
    }
}

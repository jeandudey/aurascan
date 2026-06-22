use adw::prelude::*;
use gtk::subclass::prelude::*;
use gtk::{gio, glib};

use crate::widgets::PreferencesWindow;
use crate::{Application, base_id};

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

                provider: Default::default(),
                settings: gio::Settings::new(base_id()),

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
                window.show_preferences_dialog();
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

            let obj = self.obj();

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

    impl WindowImpl for Window {}

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

    fn show_preferences_dialog(&self) {
        if self.visible_dialog().is_some() {
            return;
        }

        let preferences = PreferencesWindow::new();
        preferences.present(Some(self));
    }
}

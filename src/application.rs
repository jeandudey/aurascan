use gtk::prelude::*;
use gtk::{gio, glib};

mod imp {
    use adw::subclass::prelude::*;

    use super::*;

    #[derive(Debug, Default)]
    pub struct Application;

    #[glib::object_subclass]
    impl ObjectSubclass for Application {
        const NAME: &'static str = "AscApplication";
        type Type = super::Application;
        type ParentType = adw::Application;
    }

    impl ObjectImpl for Application {
        fn constructed(&self) {
            log::debug!("Application::constructed");
            self.parent_constructed();

            self.obj().add_main_option(
                "debug",
                glib::Char::from(b'd'),
                glib::OptionFlags::NONE,
                glib::OptionArg::None,
                "Enable debug messages",
                None,
            );
        }
    }

    impl ApplicationImpl for Application {
        fn handle_local_options(
            &self,
            options: &glib::VariantDict,
        ) -> std::ops::ControlFlow<glib::ExitCode> {
            let is_debug = options.lookup::<bool>("debug").unwrap().unwrap_or_default()
                || !glib::log_writer_default_would_drop(glib::LogLevel::Debug, Some("aurascan"));

            if is_debug {
                tracing_subscriber::fmt()
                    .with_max_level(tracing_subscriber::filter::LevelFilter::DEBUG)
                    .init();
            } else {
                tracing_subscriber::fmt::init();
            }

            log::debug!("Application::handle_local_options");

            self.parent_handle_local_options(options)
        }

        fn startup(&self) {
            log::info!("Aurascan ({})", "tech.jeandudey.Aurascan");
            self.parent_startup();

            aurascan_gtk4::init();
        }
    }

    impl GtkApplicationImpl for Application {}
    impl AdwApplicationImpl for Application {}
}

glib::wrapper! {
    pub struct Application(ObjectSubclass<imp::Application>)
        @extends gio::Application, gtk::Application, adw::Application,
        @implements gio::ActionMap, gio::ActionGroup;
}

impl Default for Application {
    fn default() -> Self {
        glib::Object::builder()
            .property("application-id", crate::app_id())
            .property("resource-base-path", "/tech/jeandudey/Aurascan/")
            .build()
    }
}

impl Application {
    pub fn new() -> Self {
        Self::default()
    }
}

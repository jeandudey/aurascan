use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::CompositeTemplate;
use gtk::{gio, glib};

use crate::config::app_id;

mod imp {
    use std::cell::{Cell, OnceCell};

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/tech/jeandudey/Aurascan/ui/preferences_window.ui")]
    #[properties(wrapper_type = super::PreferencesWindow)]
    pub struct PreferencesWindow {
        #[template_child]
        pub backend: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub output: TemplateChild<adw::ComboRow>,
        #[template_child]
        pub wine: TemplateChild<adw::PreferencesGroup>,
        #[template_child]
        pub steam_application: TemplateChild<adw::ComboRow>,

        #[property(get, set, construct_only)]
        pub is_detecting_head_pose: Cell<bool>,

        pub settings: OnceCell<gio::Settings>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PreferencesWindow {
        const NAME: &'static str = "AscPreferencesWindow";
        type Type = super::PreferencesWindow;
        type ParentType = adw::PreferencesDialog;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for PreferencesWindow {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let steam_applications = setup_steam_applications(&self.steam_application);

            let settings = gio::Settings::new(app_id());
            let action_group = gio::SimpleActionGroup::new();

            if !obj.is_detecting_head_pose() {
                let backend = settings.create_action("backend");
                action_group.add_action(&backend);

                settings
                    .bind("backend", &self.backend.get(), "selected")
                    .mapping(|variant, _| {
                        variant
                            .str()
                            .and_then(|s| match s {
                                "flex" => Some(0),
                                "vulkan" => Some(1),
                                "rocm" => Some(2),
                                _ => None,
                            })
                            .map(|idx: u32| idx.to_value())
                    })
                    .set_mapping(|value, _| {
                        let nick = match value.get::<u32>().ok()? {
                            0 => "flex",
                            1 => "vulkan",
                            2 => "rocm",
                            _ => return None,
                        };
                        Some(nick.to_variant())
                    })
                    .build();

                let output = settings.create_action("output");
                action_group.add_action(&output);

                settings
                    .bind("output", &self.output.get(), "selected")
                    .mapping(|variant, _| {
                        variant
                            .str()
                            .and_then(|s| match s {
                                "wine" => Some(0),
                                "flightgear" => Some(1),
                                "network" => Some(2),
                                _ => None,
                            })
                            .map(|idx: u32| idx.to_value())
                    })
                    .set_mapping(|value, _| {
                        let nick = match value.get::<u32>().ok()? {
                            0 => "wine",
                            1 => "flightgear",
                            2 => "network",
                            _ => return None,
                        };

                        Some(nick.to_variant())
                    })
                    .build();

                settings.connect_changed(
                    Some("output"),
                    glib::clone!(
                        #[weak]
                        obj,
                        #[weak(rename_to = wine)]
                        self.wine,
                        move |settings, _| {
                            let visible = settings.string("output") == "wine";
                            wine.set_visible(visible);
                            obj.set_content_height(-1);
                            obj.queue_resize();
                        }
                    ),
                );

                let use_steam_proton = settings.create_action("use-steam-proton");
                action_group.add_action(&use_steam_proton);

                settings
                    .bind("use-steam-proton", &self.steam_application.get(), "visible")
                    .flags(gio::SettingsBindFlags::GET)
                    .build();

                settings
                    .bind(
                        "steam-application-id",
                        &self.steam_application.get(),
                        "selected",
                    )
                    .mapping(glib::clone!(
                        #[weak]
                        steam_applications,
                        #[upgrade_or]
                        None,
                        move |variant, _ty| {
                            let id = variant.get::<u64>()?;
                            let pos = steam_applications
                                .iter::<aurascan_gtk4::SteamApplication>()
                                .position(|app| app.map(|app| app.id()) == Ok(id))
                                .unwrap_or(gtk::INVALID_LIST_POSITION as usize);
                            Some((pos as u32).to_value())
                        }
                    ))
                    .set_mapping(glib::clone!(
                        #[weak]
                        steam_applications,
                        #[upgrade_or]
                        None,
                        move |value, _ty| {
                            let pos = value.get::<u32>().ok()?;
                            steam_applications
                                .item(pos)
                                .and_downcast::<aurascan_gtk4::SteamApplication>()
                                .map(|app| app.id().to_variant())
                        }
                    ))
                    .build();
            } else {
                self.backend.set_sensitive(false);
            }

            obj.insert_action_group("preferences-window", Some(&action_group));
            self.settings.set(settings).unwrap();
        }
    }

    impl WidgetImpl for PreferencesWindow {}
    impl AdwDialogImpl for PreferencesWindow {}
    impl PreferencesDialogImpl for PreferencesWindow {}

    fn setup_steam_applications(steam_application: &adw::ComboRow) -> gio::ListStore {
        let model = gio::ListStore::new::<aurascan_gtk4::SteamApplication>();
        steam_application.set_model(Some(&model));

        let expression = gtk::PropertyExpression::new(
            aurascan_gtk4::SteamApplication::static_type(),
            None::<gtk::Expression>,
            "name",
        );
        steam_application.set_expression(Some(expression));

        let library_folders = match aurascan_steam::library_folders() {
            Ok(v) => v,
            Err(err) => {
                log::error!("Failed to parse Steam library folders: {err}");
                return model;
            }
        };

        for (id, library_folder) in library_folders.folders {
            let manifests = match library_folder.manifests() {
                Ok(v) => v,
                Err(err) => {
                    log::error!("Failed to parse Steam manifests for library folder {id}: {err}");
                    continue;
                }
            };

            for app in manifests
                .iter()
                .filter(|manifest| manifest.1.has_wine_prefix(&library_folder.path))
                .map(|(&app_id, manifest)| {
                    aurascan_gtk4::SteamApplication::new(app_id, &manifest.name)
                })
            {
                model.append(&app);
            }
        }

        model
    }
}

glib::wrapper! {
    pub struct PreferencesWindow(ObjectSubclass<imp::PreferencesWindow>)
        @extends gtk::Widget, adw::Dialog, adw::PreferencesDialog,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl PreferencesWindow {
    pub fn new(is_detecting_head_pose: bool) -> Self {
        glib::Object::builder()
            .property("is-detecting-head-pose", is_detecting_head_pose)
            .build()
    }
}

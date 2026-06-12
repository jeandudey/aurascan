use gst::glib;
use gst::prelude::DeviceExt;
use gtk::prelude::*;
use relm4::css;
use relm4::prelude::*;
use std::cell::RefCell;
use std::cmp::Reverse;
use std::sync::Arc;

#[derive(Debug)]
pub struct ResolutionSelector {
    dropdown_model: gtk::StringList,
    dropdown_factory: gtk::SignalListItemFactory,
    dropdown_list_factory: gtk::SignalListItemFactory,
    caps: Arc<RefCell<Vec<gst::Caps>>>,
    selected: Option<usize>,
}

#[derive(Debug)]
pub enum ResolutionSelectorInput {
    DeviceChanged(Option<gst::Device>),
    ResolutionSelected(u32),
}

#[relm4::component(pub)]
impl SimpleComponent for ResolutionSelector {
    type Init = ();
    type Input = ResolutionSelectorInput;
    type Output = Option<gst::Caps>;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_text: "Resolution",
                set_halign: gtk::Align::Start,
                set_margin_horizontal: 4,
            },

            gtk::DropDown {
                set_hexpand: true,
                set_model: Some(&model.dropdown_model),
                set_factory: Some(&model.dropdown_factory),
                set_list_factory: Some(&model.dropdown_list_factory),
                connect_selected_notify[sender] => move |dropdown| {
                    sender.input(ResolutionSelectorInput::ResolutionSelected(dropdown.selected()));
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let caps: Arc<RefCell<Vec<gst::Caps>>> = Arc::new(RefCell::new(Vec::new()));

        let dropdown_model = gtk::StringList::new(&["Not available"]);

        let dropdown_factory = gtk::SignalListItemFactory::new();
        dropdown_factory.connect_setup(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let label = gtk::Label::builder()
                .hexpand(true)
                .halign(gtk::Align::Start)
                .ellipsize(gtk::pango::EllipsizeMode::End)
                .build();
            list_item.set_child(Some(&label));
        });
        dropdown_factory.connect_bind(glib::clone!(
            #[strong]
            caps,
            move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                if let Some(caps) = caps.borrow().get(list_item.position() as usize) {
                    let label = list_item.child().unwrap().downcast::<gtk::Label>().unwrap();

                    let (width, height) = resolution(caps);
                    label.set_label(&format!("{}x{}", width, height));
                }
            }
        ));

        let dropdown_list_factory = gtk::SignalListItemFactory::new();
        dropdown_list_factory.connect_setup(|_, list_item| {
            let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
            let template = ResolutionListItem::init(());
            list_item.set_child(Some(template.as_ref()));
        });
        dropdown_list_factory.connect_bind(glib::clone!(
            #[strong]
            caps,
            move |_, list_item| {
                let list_item = list_item.downcast_ref::<gtk::ListItem>().unwrap();
                if let Some(caps) = caps.borrow().get(list_item.position() as usize) {
                    let (width, height) = resolution(caps);

                    let structure = caps.structure(0).unwrap();
                    let format = structure
                        .get::<String>("drm-format")
                        .map(|drm_format| format!("{} (DRM)", drm_format))
                        .unwrap_or_else(|_| {
                            structure.get::<String>("format").unwrap_or_else(|_| {
                                let name = structure.name();
                                if name == "image/jpeg" {
                                    "MJPEG".to_string()
                                } else {
                                    name.to_string()
                                }
                            })
                        });

                    let child = list_item.child().unwrap();
                    let resolution_label = child
                        .first_child()
                        .unwrap()
                        .downcast::<gtk::Label>()
                        .unwrap();
                    let format_label = resolution_label
                        .next_sibling()
                        .unwrap()
                        .downcast::<gtk::Label>()
                        .unwrap();

                    resolution_label.set_label(&format!("{}x{}", width, height));
                    format_label.set_label(&format);
                };
            }
        ));

        let model = Self {
            dropdown_model,
            dropdown_factory,
            dropdown_list_factory,
            caps,
            selected: None,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ResolutionSelectorInput::DeviceChanged(device) => {
                clear_string_list(&self.dropdown_model);

                let Some(device_caps) = device.and_then(|device| device.caps()) else {
                    self.caps.borrow_mut().clear();
                    self.dropdown_model.append("Not available");
                    return;
                };

                let caps = individual_caps(&device_caps);
                if caps.is_empty() {
                    self.caps.borrow_mut().clear();
                    self.dropdown_model.append("Not available");
                    return;
                }

                let labels = caps
                    .iter()
                    .map(|caps| {
                        let (width, height) = resolution(caps);
                        format!("{}x{}", width, height)
                    })
                    .collect::<Vec<_>>();

                *self.caps.borrow_mut() = caps;

                for label in labels {
                    self.dropdown_model.append(&label);
                }

                if self.selected.is_none() && !self.caps.borrow().is_empty() {
                    self.selected = Some(0);
                    sender
                        .output(self.caps.borrow().first().map(|caps| caps.copy()))
                        .ok();
                }
            }
            ResolutionSelectorInput::ResolutionSelected(idx) => {
                if idx == gtk::INVALID_LIST_POSITION {
                    return;
                }

                let idx = idx as usize;
                if self.selected != Some(idx) {
                    self.selected = Some(idx);
                    sender.output(self.caps.borrow_mut().get(idx).cloned()).ok();
                }
            }
        }
    }
}

#[relm4::widget_template(pub)]
impl WidgetTemplate for ResolutionListItem {
    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,
            set_hexpand: true,

            gtk::Label {
                set_hexpand: true,
                set_halign: gtk::Align::Start,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
            },

            gtk::Label {
                set_hexpand: true,
                set_halign: gtk::Align::Start,
                set_ellipsize: gtk::pango::EllipsizeMode::End,
                add_css_class: css::DIM_LABEL,
                add_css_class: css::CAPTION,
            },
        }
    }
}

/// Split the device caps into individual resolution caps sorted by
/// resolution.
fn individual_caps(caps: &gst::Caps) -> Vec<gst::Caps> {
    let mut individual_caps = caps
        .iter()
        .filter_map(|structure| {
            // XXX: Skip DRM formats until I learn how to handle it.
            if structure.has_field("drm-format") {
                return None;
            }

            structure.get::<i32>("width").ok()?;
            structure.get::<i32>("height").ok()?;

            let mut caps = gst::Caps::new_empty();
            caps.get_mut()
                .unwrap()
                .append_structure(structure.to_owned());
            Some(caps)
        })
        .collect::<Vec<_>>();
    individual_caps.sort_by_key(|caps| {
        let (width, height) = resolution(caps);
        Reverse(width * height)
    });
    individual_caps
}

fn resolution(caps: &gst::Caps) -> (i32, i32) {
    let structure = caps.structure(0).expect("should have structure");
    let width = structure.get::<i32>("width").expect("should have width");
    let height = structure.get::<i32>("height").expect("should have height");
    (width, height)
}

fn clear_string_list(string_list: &gtk::StringList) {
    while string_list.n_items() > 0 {
        string_list.remove(0);
    }
}

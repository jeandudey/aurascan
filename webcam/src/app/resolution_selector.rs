use gst::prelude::DeviceExt;
use gtk::prelude::*;
use relm4::prelude::*;
use std::cmp::Reverse;
use std::fmt::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

impl Display for Resolution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}x{}", self.width, self.height)
    }
}

#[derive(Debug)]
pub struct ResolutionSelector {
    string_list: gtk::StringList,
    resolutions: Vec<Resolution>,
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
    type Output = Option<Resolution>;

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
                set_model: Some(&model.string_list),
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
        let model = Self {
            string_list: gtk::StringList::new(&["Not available"]),
            resolutions: Vec::new(),
            selected: None,
        };
        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            ResolutionSelectorInput::DeviceChanged(device) => {
                clear_string_list(&self.string_list);

                let Some(device) = device else {
                    self.string_list.append("Not available");
                    return;
                };

                self.resolutions = device_resolutions(&device);
                if self.resolutions.is_empty() {
                    self.string_list.append("Not available");
                    return;
                }

                for resolution in self.resolutions.iter() {
                    self.string_list.append(&resolution.to_string());
                }

                if self.selected.is_none() && !self.resolutions.is_empty() {
                    self.selected = Some(0);
                    sender
                        .output(self.resolutions.first().map(|device| device.clone()))
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
                    println!("Resolution change: {:?}", self.resolutions.get(idx));
                    sender.output(self.resolutions.get(idx).cloned()).ok();
                }
            }
        }
    }
}

fn device_resolutions(device: &gst::Device) -> Vec<Resolution> {
    const CUTOFF: Resolution = Resolution {
        width: 640,
        height: 640,
    };

    let Some(caps) = device.caps() else {
        return Vec::new();
    };

    let mut resolutions = caps
        .iter()
        .filter_map(|structure| {
            Some(Resolution {
                width: structure.get("width").ok()?,
                height: structure.get("height").ok()?,
            })
        })
        .collect::<Vec<_>>();
    resolutions.sort_by_key(|r| Reverse(r.width * r.height));
    resolutions.dedup();

    let has_higher = resolutions
        .iter()
        .any(|r| r.width >= CUTOFF.width && r.height >= CUTOFF.height);

    if has_higher {
        resolutions.retain(|r| r.width >= CUTOFF.width && r.height >= CUTOFF.height);
    }

    resolutions
}

fn clear_string_list(string_list: &gtk::StringList) {
    while string_list.n_items() > 0 {
        string_list.remove(0);
    }
}

use gst::glib;
use gst::prelude::*;
use gtk::prelude::*;
use relm4::prelude::*;
use relm4::{ComponentParts, ComponentSender, Sender};

pub struct SourceSelector {
    monitor: gst::DeviceMonitor,
    watch_guard: Option<gst::bus::BusWatchGuard>,
    devices: Vec<gst::Device>,
    string_list: gtk::StringList,
    selected: Option<usize>,
}

#[derive(Debug)]
pub enum SourceSelectorInput {
    DeviceAdded(gst::Device),
    DeviceRemoved(gst::Device),
    DeviceSelected(u32),
}

#[relm4::component(pub)]
impl SimpleComponent for SourceSelector {
    type Init = ();
    type Input = SourceSelectorInput;
    type Output = Option<gst::Device>;

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Vertical,

            gtk::Label {
                set_label: "Source",
                set_halign: gtk::Align::Start,
                set_margin_horizontal: 4,
            },

            gtk::DropDown {
                set_hexpand: true,
                set_model: Some(&model.string_list),
                connect_selected_notify[sender] => move |dropdown| {
                    sender.input(SourceSelectorInput::DeviceSelected(dropdown.selected()));
                }
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let monitor = gst::DeviceMonitor::new();
        monitor.add_filter(Some("Video/Source"), None);

        let bus = monitor.bus();
        let watch_guard = bus
            .add_watch_local({
                let sender = sender.clone();
                move |_, msg| {
                    match msg.view() {
                        gst::MessageView::DeviceAdded(e) => {
                            sender.input(SourceSelectorInput::DeviceAdded(e.device()))
                        }
                        gst::MessageView::DeviceRemoved(e) => {
                            sender.input(SourceSelectorInput::DeviceRemoved(e.device()));
                        }
                        _ => {}
                    }

                    glib::ControlFlow::Continue
                }
            })
            .unwrap();
        monitor.start().unwrap();

        let model = SourceSelector {
            monitor,
            watch_guard: Some(watch_guard),
            devices: Vec::new(),
            string_list: gtk::StringList::new(&[]),
            selected: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            SourceSelectorInput::DeviceAdded(device) => {
                self.devices.push(device);
                self.rebuild_list();
                if self.selected.is_none() && !self.devices.is_empty() {
                    self.selected = Some(0);
                    sender
                        .output(self.devices.first().map(|device| device.clone()))
                        .ok();
                }
            }
            SourceSelectorInput::DeviceRemoved(device) => {
                if let Some(pos) = self.devices.iter().position(|d| d == &device) {
                    let was_selected = self.selected == Some(pos);
                    self.devices.remove(pos);
                    self.rebuild_list();

                    self.selected = match self.selected {
                        Some(s) if s == pos => {
                            if self.devices.is_empty() {
                                None
                            } else {
                                Some(0)
                            }
                        }
                        Some(s) if s > pos => Some(s - 1),
                        other => other,
                    };

                    if was_selected {
                        sender
                            .output(
                                self.selected
                                    .and_then(|i| self.devices.get(i).map(|device| device.clone())),
                            )
                            .ok();
                    }
                }
            }
            SourceSelectorInput::DeviceSelected(idx) => {
                if idx == gtk::INVALID_LIST_POSITION {
                    return;
                }

                let idx = idx as usize;
                if self.selected != Some(idx) {
                    self.selected = Some(idx);
                    sender
                        .output(self.devices.get(idx).map(|device| device.clone()))
                        .ok();
                }
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: Sender<Self::Output>) {
        drop(self.watch_guard.take());
        self.monitor.stop();
    }
}

impl SourceSelector {
    fn rebuild_list(&self) {
        while self.string_list.n_items() > 0 {
            self.string_list.remove(0);
        }

        for entry in self.devices.iter() {
            self.string_list
                .append(&gst::prelude::DeviceExt::display_name(entry));
        }
    }
}

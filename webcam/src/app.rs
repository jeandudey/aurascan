use crate::app::msg::AppMsg;
use crate::app::resolution_selector::{ResolutionSelector, ResolutionSelectorInput};
use crate::app::source_selector::SourceSelector;
use crate::app::status::{Status, StatusInput};
use crate::pipeline::{Pipeline, PipelineState};
use adw::prelude::*;
use relm4::SimpleComponent;
use relm4::prelude::*;
use std::sync::{Arc, Mutex};

pub(crate) mod msg;
pub(crate) mod resolution_selector;
pub(crate) mod source_selector;
pub(crate) mod status;

pub struct AppModel {
    resolution_selector: relm4::Controller<ResolutionSelector>,
    source_selector: relm4::Controller<SourceSelector>,
    status: relm4::Controller<Status>,
    pipeline: Arc<Mutex<Pipeline>>,
    toggle_label: &'static str,
    start_requested: bool,
}

#[relm4::component(pub)]
impl SimpleComponent for AppModel {
    type Init = ();
    type Input = AppMsg;
    type Output = ();

    view! {
        #[root]
        adw::ApplicationWindow::builder()
            .default_width(1280)
            .default_height(720)
            .title("Aura Scan")
            .build() {
            adw::ToastOverlay {
                gtk::Box {
                    set_orientation: gtk::Orientation::Vertical,

                    adw::NavigationSplitView {
                        set_expand: true,
                        set_max_sidebar_width: 230.0,

                        #[wrap(Some)]
                        set_sidebar = &adw::NavigationPage {
                            set_title: "Aura Scan",

                            #[wrap(Some)]
                            set_child = &adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {
                                    set_show_end_title_buttons: false,
                                },

                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,
                                    set_vexpand: true,
                                    set_spacing: 32,
                                    set_margin_start: 8,
                                    set_margin_end: 8,
                                    set_margin_top: 4,
                                    set_margin_bottom: 4,

                                    model.source_selector.widget(),

                                    model.resolution_selector.widget(),

                                    gtk::Box {
                                        set_orientation: gtk::Orientation::Vertical,

                                        gtk::Label {
                                            set_label: "Backend",
                                            set_halign: gtk::Align::Start,
                                            set_margin_horizontal: 4,
                                        },

                                        gtk::DropDown {
                                            set_hexpand: true,
                                            set_model: Some(&gtk::StringList::new(&["Flex (CPU)", "Vulkan", "ROCm"])),
                                            connect_selected_notify[sender] => move |dropdown| {
                                                sender.input(AppMsg::BackendSelected(dropdown.selected()));
                                            }
                                        },
                                    },

                                    gtk::Separator {},

                                    gtk::Box {
                                        set_vexpand: true,
                                    },

                                    gtk::Separator {},

                                    gtk::Button {
                                        #[watch]
                                        set_label: model.toggle_label,
                                        set_width_request: 150,
                                        connect_clicked => AppMsg::TogglePipeline,
                                    },
                                },
                            },
                        },

                        #[wrap(Some)]
                        set_content = &adw::NavigationPage {
                            set_title: "Live Feed",

                            #[wrap(Some)]
                            set_child = &adw::ToolbarView {
                                add_top_bar = &adw::HeaderBar {},

                                #[wrap(Some)]
                                set_content = &gtk::Box {
                                    set_orientation: gtk::Orientation::Vertical,

                                    gtk::Overlay {
                                        #[wrap(Some)]
                                        set_child = &gstgtk4::RenderWidget::new(&model.pipeline.lock().unwrap().sink()) {
                                            set_hexpand: true,
                                            set_vexpand: true,
                                        },

                                        add_overlay = &gtk::Spinner {
                                            set_halign: gtk::Align::Center,
                                            set_valign: gtk::Align::Center,
                                            set_width_request: 48,
                                            set_height_request: 48,
                                            #[watch]
                                            set_visible: model.start_requested,
                                            #[watch]
                                            set_spinning: model.start_requested,
                                        },
                                    },

                                    model.status.widget(),
                                }
                            },
                        },
                    },
                },
            },
        }
    }

    fn init(
        _init: Self::Init,
        root: Self::Root,
        sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let resolution_selector = ResolutionSelector::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::ResolutionSelected);

        let source_selector =
            SourceSelector::builder()
                .launch(())
                .forward(sender.input_sender(), {
                    let resolutions_sender = resolution_selector.sender().clone();
                    move |device| {
                        resolutions_sender
                            .send(ResolutionSelectorInput::DeviceChanged(device.clone()))
                            .unwrap();
                        AppMsg::SourceChanged(device)
                    }
                });

        let status = Status::builder().launch(()).detach();

        let mut pipeline = Pipeline::new().unwrap();

        pipeline.connect_state_changed({
            let status_sender = sender.input_sender().clone();
            move |state| {
                status_sender
                    .send(AppMsg::PipelineStateChanged(state))
                    .unwrap();
            }
        });

        pipeline.connect_fps_measurements({
            let status_sender = status.sender().clone();
            move |fps, droprate, avgfps| {
                status_sender
                    .send(StatusInput::UpdateFps {
                        fps,
                        droprate,
                        avgfps,
                    })
                    .unwrap();
            }
        });

        pipeline.connect_inference_measurements({
            let status_sender = status.sender().clone();
            move |measurements| {
                status_sender
                    .send(StatusInput::UpdateInference(measurements))
                    .unwrap();
            }
        });

        let model = AppModel {
            resolution_selector,
            source_selector,
            status,
            pipeline: Arc::new(Mutex::new(pipeline)),
            toggle_label: "Start",
            start_requested: false,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, sender: ComponentSender<Self>) {
        match message {
            AppMsg::SourceChanged(device) => {
                sender.oneshot_command({
                    let pipeline = self.pipeline.clone();
                    async move {
                        pipeline.lock().unwrap().set_source(device).unwrap();
                    }
                });
            }
            AppMsg::TogglePipeline => {
                let pipeline = self.pipeline.lock().unwrap();
                if pipeline.is_playing() {
                    self.start_requested = false;
                    self.toggle_label = "Start";
                    self.status.sender().send(StatusInput::Clear).unwrap();

                    sender.oneshot_command({
                        let pipeline = self.pipeline.clone();
                        async move {
                            pipeline.lock().unwrap().stop().unwrap();
                        }
                    });
                } else {
                    self.start_requested = true;
                    sender.oneshot_command({
                        let pipeline = self.pipeline.clone();
                        async move {
                            pipeline.lock().unwrap().play().unwrap();
                        }
                    });
                }
            }
            AppMsg::PipelineStateChanged(state) => match (state, self.start_requested) {
                (PipelineState::Started, true) => {
                    self.toggle_label = "Stop";
                    self.start_requested = false;
                }
                _ => (),
            },
            AppMsg::BackendSelected(backend_type) => {
                sender.oneshot_command({
                    let pipeline = self.pipeline.clone();
                    async move {
                        pipeline
                            .lock()
                            .unwrap()
                            .set_backend_type(match backend_type {
                                0 => gstburnextra::BackendType::Flex,
                                1 => gstburnextra::BackendType::Vulkan,
                                2 => gstburnextra::BackendType::Rocm,
                                _ => unreachable!(),
                            })
                            .unwrap();
                    }
                });
            }
            AppMsg::ResolutionSelected(resolution) => {
                let Some(resolution) = resolution else {
                    return;
                };

                sender.oneshot_command({
                    let pipeline = self.pipeline.clone();
                    async move {
                        pipeline.lock().unwrap().set_resolution(resolution).unwrap();
                    }
                });
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.pipeline.lock().unwrap().stop().unwrap();
    }
}

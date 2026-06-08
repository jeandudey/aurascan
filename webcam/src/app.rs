use crate::app::msg::AppMsg;
use crate::app::source_selector::SourceSelector;
use crate::app::status::{Status, StatusInput};
use crate::pipeline::Pipeline;
use adw::prelude::*;
use relm4::SimpleComponent;
use relm4::prelude::*;

mod msg;
mod source_selector;
mod status;

pub struct AppModel {
    source_selector: relm4::Controller<SourceSelector>,
    status: relm4::Controller<Status>,
    pipeline: Pipeline,
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

                                        gtk::Box {
                                            set_orientation: gtk::Orientation::Vertical,

                                            gtk::Label {
                                                set_label: "Backend",
                                                set_halign: gtk::Align::Start,
                                                set_margin_horizontal: 4,
                                            },

                                            gtk::DropDown {
                                                set_hexpand: true,
                                                set_model: Some(&gtk::StringList::new(&["Vulkan", "NdArray"])),
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

                                        #[name(start)]
                                        gtk::Button {
                                            #[watch]
                                            set_label: if model.pipeline.is_playing() { "Stop" } else { "Start" },
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

                                        gstgtk4::RenderWidget::new(&model.pipeline.sink()) {
                                            set_hexpand: true,
                                            set_vexpand: true,
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
        let source_selector = SourceSelector::builder()
            .launch(())
            .forward(sender.input_sender(), AppMsg::SourceChanged);

        let status = Status::builder().launch(()).detach();

        let pipeline = Pipeline::new().unwrap();

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
            source_selector,
            status,
            pipeline,
        };
        let widgets = view_output!();
        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            AppMsg::SourceChanged(device) => {
                self.pipeline.set_source(device).unwrap();
            }
            AppMsg::TogglePipeline => {
                if self.pipeline.is_playing() {
                    self.pipeline.stop().unwrap();
                } else {
                    self.pipeline.play().unwrap();
                }
            }
            AppMsg::BackendSelected(backend_type) => {
                self.pipeline
                    .set_backend_type(match backend_type {
                        0 => gstburnextra::BackendType::Vulkan,
                        1 => gstburnextra::BackendType::NdArray,
                        _ => unreachable!(),
                    })
                    .unwrap();
            }
        }
    }

    fn shutdown(&mut self, _widgets: &mut Self::Widgets, _output: relm4::Sender<Self::Output>) {
        self.pipeline.stop().unwrap();
    }
}

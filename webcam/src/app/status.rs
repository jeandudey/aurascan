use gtk::prelude::*;
use relm4::prelude::*;

pub struct Status {
    pub fps: Option<f64>,
    pub droprate: Option<f64>,
    pub avgfps: Option<f64>,
}

#[derive(Debug)]
pub enum StatusInput {
    UpdateFps {
        fps: f64,
        droprate: f64,
        avgfps: f64,
    },
    Clear,
}

#[relm4::component(pub)]
impl SimpleComponent for Status {
    type Init = ();
    type Input = StatusInput;
    type Output = ();

    view! {
        gtk::Box {
            set_orientation: gtk::Orientation::Horizontal,
            set_spacing: 32,
            set_margin_start: 8,
            set_margin_end: 8,
            set_margin_top: 4,
            set_margin_bottom: 4,

            gtk::Label {
                #[watch]
                set_label: &measurement("FPS", model.fps),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Avg. FPS", model.avgfps),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Drop Rate", model.droprate),
            },
        }
    }

    fn init(
        _init: Self::Init,
        _root: Self::Root,
        _sender: ComponentSender<Self>,
    ) -> ComponentParts<Self> {
        let model = Status {
            fps: None,
            droprate: None,
            avgfps: None,
        };

        let widgets = view_output!();

        ComponentParts { model, widgets }
    }

    fn update(&mut self, message: Self::Input, _sender: ComponentSender<Self>) {
        match message {
            StatusInput::UpdateFps {
                fps,
                droprate,
                avgfps,
            } => {
                self.fps = Some(fps);
                self.droprate = Some(droprate);
                self.avgfps = Some(avgfps);
            }
            StatusInput::Clear => {
                self.fps = None;
                self.droprate = None;
                self.avgfps = None;
            }
        }
    }
}

fn measurement(name: &str, value: Option<f64>) -> String {
    match value {
        Some(v) => format!("{name}: {:.2}", v),
        None => format!("{name}: --"),
    }
}

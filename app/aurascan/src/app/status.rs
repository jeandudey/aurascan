use crate::pipeline::InferenceMeasurements;
use gtk::prelude::*;
use relm4::prelude::*;

pub struct Status {
    pub fps: Option<f64>,
    pub droprate: Option<f64>,
    pub avgfps: Option<f64>,
    pub inference: Option<InferenceMeasurements>,
}

#[derive(Debug)]
pub enum StatusInput {
    UpdateFps {
        fps: f64,
        droprate: f64,
        avgfps: f64,
    },
    UpdateInference(InferenceMeasurements),
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
                set_label: &measurement("FPS", model.fps, None),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Avg. FPS", model.avgfps, None),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Drop Rate", model.droprate, None),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("X", model.inference.as_ref().map(|m| m.x as f64), Some("mm")),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Y", model.inference.as_ref().map(|m| m.y as f64), Some("mm")),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Z", model.inference.as_ref().map(|m| m.z as f64), Some("mm")),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Yaw", model.inference.as_ref().map(|m| m.yaw as f64), Some("°")),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Pitch", model.inference.as_ref().map(|m| m.pitch as f64), Some("°")),
            },

            gtk::Label {
                #[watch]
                set_label: &measurement("Roll", model.inference.as_ref().map(|m| m.roll as f64), Some("°")),
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
            inference: None,
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
            StatusInput::UpdateInference(measurements) => {
                self.inference = Some(measurements);
            }
            StatusInput::Clear => {
                self.fps = None;
                self.droprate = None;
                self.avgfps = None;
                self.inference = None;
            }
        }
    }
}

fn measurement(name: &str, value: Option<f64>, unit: Option<&str>) -> String {
    match (value, unit) {
        (Some(v), None) => format!("{name}: {v:.2}"),
        (Some(v), Some(u)) => format!("{name}: {v:.2} {u}"),
        (None, Some(u)) => format!("{name}: -- {u}"),
        (None, None) => format!("{name}: --"),
    }
}

use crate::pipeline2::{InferenceMeasurements, PipelineState};

#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    ToggleDetect,
    PipelineStateChanged(PipelineState),
    UpdateInference(InferenceMeasurements),
    UpdateFps {
        fps: f64,
        droprate: f64,
        avgfps: f64,
    },
    SetBackend(u32),
    SetCaps(Option<gst::Caps>),
    Error(String),
    HideError,
}

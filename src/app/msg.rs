use crate::pipeline2::{InferenceMeasurements, PipelineState};

#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    TogglePipeline,
    PipelineStateChanged(PipelineState),
    UpdateInference(InferenceMeasurements),
    SetBackend(u32),
    SetCaps(Option<gst::Caps>),
    Error(String),
    HideError,
}

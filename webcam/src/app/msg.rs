use crate::{app::resolution_selector::Resolution, pipeline::PipelineState};

#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    TogglePipeline,
    PipelineStateChanged(PipelineState),
    BackendSelected(u32),
    ResolutionSelected(Option<Resolution>),
    Error(String),
    HideError,
}

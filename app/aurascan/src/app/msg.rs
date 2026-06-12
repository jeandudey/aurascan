use crate::pipeline::PipelineState;

#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    TogglePipeline,
    PipelineStateChanged(PipelineState),
    SetBackend(u32),
    SetCaps(Option<gst::Caps>),
    Error(String),
    HideError,
}

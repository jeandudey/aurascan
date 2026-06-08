use crate::pipeline::PipelineState;

#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    TogglePipeline,
    PipelineStateChanged(PipelineState),
    BackendSelected(u32),
}

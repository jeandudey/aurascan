#[derive(Debug)]
pub enum AppMsg {
    SourceChanged(Option<gst::Device>),
    TogglePipeline,
}

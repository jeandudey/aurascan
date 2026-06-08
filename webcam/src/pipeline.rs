use gst::glib;
use gst::prelude::*;

pub struct Pipeline {
    pipeline: gst::Pipeline,
    source: Option<gst::Element>,
    after_source: gst::Element,
    fpsdisplaysink: gst::Element,
    sink: gst::Element,
}

impl Pipeline {
    pub fn new() -> Result<Self, glib::BoolError> {
        let pipeline = gst::Pipeline::new();

        let caps = gst_video::VideoCapsBuilder::new()
            .format(gst_video::VideoFormat::Rgb)
            .width(640)
            .height(640)
            .build();

        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property("caps", &caps)
            .build()?;

        let videoconvertscale = gst::ElementFactory::make("videoconvertscale")
            .property("add-borders", true)
            .build()?;
        let inferencebin = gst::ElementFactory::make("headposeinferencebin").build()?;
        let videoconvert1 = gst::ElementFactory::make("videoconvert").build()?;

        let sink = gst::ElementFactory::make("gtk4paintablesink").build()?;

        let fpsdisplaysink = gst::ElementFactory::make("fpsdisplaysink")
            .property("text-overlay", false)
            .property("signal-fps-measurements", true)
            .property("video-sink", &sink)
            .property("sync", false)
            .build()?;

        pipeline.add_many([
            &videoconvertscale,
            &capsfilter,
            &inferencebin,
            &videoconvert1,
            &fpsdisplaysink,
        ])?;
        gst::Element::link_many([
            &videoconvertscale,
            &capsfilter,
            &inferencebin,
            &videoconvert1,
            &fpsdisplaysink,
        ])?;

        Ok(Self {
            pipeline,
            source: None,
            after_source: videoconvertscale,
            fpsdisplaysink,
            sink,
        })
    }

    pub fn is_playing(&self) -> bool {
        self.pipeline.current_state() == gst::State::Playing
    }

    pub fn play(&self) -> Result<(), gst::StateChangeError> {
        if self.source.is_none() {
            return Ok(());
        }

        self.pipeline.set_state(gst::State::Playing)?;

        Ok(())
    }

    pub fn stop(&self) -> Result<(), gst::StateChangeError> {
        self.pipeline.set_state(gst::State::Null)?;
        Ok(())
    }

    pub fn connect_fps_measurements<F>(&self, callback: F) -> glib::SignalHandlerId
    where
        F: Fn(f64, f64, f64) + Send + Sync + 'static,
    {
        self.fpsdisplaysink
            .connect("fps-measurements", false, move |args| {
                let fps = args[1].get().unwrap();
                let droprate = args[2].get().unwrap();
                let avgfps = args[3].get().unwrap();
                callback(fps, droprate, avgfps);
                None
            })
    }

    pub fn set_source(&mut self, device: Option<gst::Device>) -> Result<(), glib::BoolError> {
        let was_playing = self.is_playing();

        if self.source.is_some() {
            self.remove_source()?
        }

        let Some(device) = device else {
            return Ok(());
        };

        let source = device.create_element(None)?;
        self.pipeline.add(&source)?;
        source.link(&self.after_source)?;
        source.sync_state_with_parent()?;
        self.source = Some(source);

        if was_playing {
            self.play().unwrap();
        }

        Ok(())
    }

    pub fn remove_source(&mut self) -> Result<(), glib::BoolError> {
        self.stop().unwrap();

        match self.source.take() {
            Some(source) => {
                source.unlink(&self.after_source);
                self.pipeline.remove(&source)?;
            }
            None => (),
        }

        return Ok(());
    }

    pub fn sink(&self) -> &gst::Element {
        &self.sink
    }
}

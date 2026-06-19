use gst::glib;
use gst::prelude::*;

#[derive(Debug)]
pub enum PipelineState {
    Started,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct InferenceMeasurements {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

pub struct Pipeline {
    pipeline: gst::Pipeline,
    pipeline_state_guard: Option<gst::bus::BusWatchGuard>,
    source: Option<gst::Element>,
    input_capsfilter: gst::Element,
    after_source: gst::Element,
    inferencebin: gst::Element,
    fpsdisplaysink: gst::Element,
    inferencesink: gst::Element,
    livefeedsink: gst::Element,
}

impl Pipeline {
    pub fn new() -> Result<Self, glib::BoolError> {
        let pipeline = gst::Pipeline::new();

        let input_caps = gst_video::VideoCapsBuilder::new()
            .pixel_aspect_ratio(gst::Fraction::new(1, 1))
            .build();

        let input_capsfilter = gst::ElementFactory::make("capsfilter")
            .property("caps", &input_caps)
            .build()?;

        let inference_caps = gst_video::VideoCapsBuilder::new()
            .format(gst_video::VideoFormat::Rgb)
            .width(640)
            .height(640)
            .pixel_aspect_ratio(gst::Fraction::new(1, 1))
            .build();

        let inference_capsfilter = gst::ElementFactory::make("capsfilter")
            .property("caps", &inference_caps)
            .build()?;

        let videoconvertscale = gst::ElementFactory::make("videoconvertscale")
            .property("add-borders", true)
            .build()?;

        let tee = gst::ElementFactory::make("tee").build()?;
        let queue_inference = gst::ElementFactory::make("queue")
            .property_from_str("leaky", "downstream")
            .property("max-size-buffers", 1u32)
            .property("max-size-bytes", 0u32)
            .property("max-size-time", 0u64)
            .build()?;
        let queue_livefeed = gst::ElementFactory::make("queue").build()?;

        let inferencebin = gst::ElementFactory::make("headposeinferencebin").build()?;

        let inferencesink = gst::ElementFactory::make("gtk4paintablesink").build()?;
        let livefeedsink = gst::ElementFactory::make("gtk4paintablesink")
            .property("sync", false)
            .build()?;

        let fpsdisplaysink = gst::ElementFactory::make("fpsdisplaysink")
            .property("text-overlay", false)
            .property("signal-fps-measurements", true)
            .property("video-sink", &inferencesink)
            .property("sync", false)
            .build()?;

        pipeline.add_many([
            &input_capsfilter,
            &videoconvertscale,
            &inference_capsfilter,
            &tee,
            &queue_inference,
            &queue_livefeed,
            &inferencebin,
            &fpsdisplaysink,
            &livefeedsink,
        ])?;
        gst::Element::link_many([
            &input_capsfilter,
            &videoconvertscale,
            &inference_capsfilter,
            &tee,
        ])?;

        gst::Element::link_many([&queue_inference, &inferencebin, &fpsdisplaysink])?;
        gst::Element::link_many([&queue_livefeed, &livefeedsink])?;

        let tee_src0 = tee.request_pad_simple("src_%u").unwrap();
        let queue0_sink = queue_inference.static_pad("sink").unwrap();
        tee_src0.link(&queue0_sink).unwrap();

        let tee_src1 = tee.request_pad_simple("src_%u").unwrap();
        let queue1_sink = queue_livefeed.static_pad("sink").unwrap();
        tee_src1.link(&queue1_sink).unwrap();

        Ok(Self {
            pipeline,
            pipeline_state_guard: None,
            source: None,
            input_capsfilter: input_capsfilter.clone(),
            after_source: input_capsfilter,
            inferencebin,
            fpsdisplaysink,
            inferencesink,
            livefeedsink,
        })
    }

    pub fn set_backend_type(
        &self,
        backend_type: gstaurascan::BackendType,
    ) -> Result<(), gst::StateChangeError> {
        let was_playing = self.is_playing();
        if was_playing {
            self.stop()?;
        }

        self.inferencebin
            .dynamic_cast_ref::<gst::ChildProxy>()
            .unwrap()
            .set_child_property("scrfdinference::backend-type", backend_type);

        self.inferencebin
            .dynamic_cast_ref::<gst::ChildProxy>()
            .unwrap()
            .set_child_property("sixdrepnet360inference::backend-type", backend_type);

        if was_playing {
            self.play()?;
        }

        Ok(())
    }

    pub fn set_caps(&self, caps: gst::Caps) -> Result<(), gst::StateChangeError> {
        let was_playing = self.is_playing();
        if was_playing {
            self.stop()?;
        }

        self.input_capsfilter.set_property("caps", &caps);

        if was_playing {
            self.play()?;
        }

        Ok(())
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

    pub fn connect_inference_measurements<F>(&self, callback: F)
    where
        F: Fn(InferenceMeasurements) + Send + Sync + 'static,
    {
        let pad = self.inferencebin.static_pad("src").unwrap();
        pad.add_probe(gst::PadProbeType::BUFFER, move |_pad, info| {
            let Some(buffer) = info.buffer() else {
                return gst::PadProbeReturn::Ok;
            };

            let Some(meta) = gst::meta::CustomMeta::from_buffer(buffer, "EulerAnglesMeta").ok()
            else {
                return gst::PadProbeReturn::Ok;
            };

            let Some((yaw, pitch, roll)) = parse_euler_angles_meta(meta) else {
                return gst::PadProbeReturn::Ok;
            };
            let measurements = InferenceMeasurements {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                yaw,
                pitch,
                roll,
            };
            callback(measurements);

            gst::PadProbeReturn::Ok
        });
    }

    pub fn connect_state_changed<F>(&mut self, callback: F)
    where
        F: Fn(PipelineState) + Send + Sync + 'static,
    {
        let bus = self.pipeline.bus().unwrap();
        let pipeline_state_guard = bus
            .add_watch(move |_, message| {
                match message.view() {
                    gst::MessageView::StateChanged(s) => match s.current() {
                        gst::State::Playing => callback(PipelineState::Started),
                        gst::State::Null => callback(PipelineState::Stopped),
                        _ => (),
                    },
                    gst::MessageView::Eos(_) => callback(PipelineState::Stopped),
                    _ => (),
                }

                glib::ControlFlow::Continue
            })
            .unwrap();
        self.pipeline_state_guard = Some(pipeline_state_guard);
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

    pub fn livefeedsink(&self) -> &gst::Element {
        &self.livefeedsink
    }

    pub fn inferencesink(&self) -> &gst::Element {
        &self.inferencesink
    }
}

fn parse_euler_angles_meta(meta: gst::MetaRef<gst::meta::CustomMeta>) -> Option<(f32, f32, f32)> {
    let structure = meta.structure();

    let yaw = structure.get::<f32>("yaw").ok()?;
    let pitch = structure.get::<f32>("pitch").ok()?;
    let roll = structure.get::<f32>("roll").ok()?;
    Some((yaw, pitch, roll))
}

use burn::prelude::*;
use scrfd_burn::Face;
use scrfd_burn::scrfd_500m::Model as Scrfd500m;
use sixdrepnet360_burn::sixdrepnet360::SixDRepNet360;
use thiserror::Error;

#[derive(Debug)]
pub struct FaceDetectorConfig {
    pub score_threshold: f32,
    pub nms_threshold: f32,
}

pub struct FaceAnalysis<B: Backend> {
    scrfd_500m: Scrfd500m<B>,
    sixdrepnet360: SixDRepNet360<B>,
}

impl<B: Backend> FaceAnalysis<B> {
    pub fn from_embedded(device: &B::Device) -> Self {
        Self {
            scrfd_500m: Scrfd500m::from_embedded(device),
            sixdrepnet360: SixDRepNet360::pretrained(device).unwrap(), // TODO: Embed.
        }
    }
}

impl<B: Backend> FaceAnalysis<B> {
    pub fn infer(&self, image: Tensor<B, 4>) -> Vec<Face> {
        Vec::new()
    }
}

#[derive(Error)]
pub enum Error {}

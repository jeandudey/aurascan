//! # SCRFD models.
//!
//! This currently is implemented using burn-onnx to save time as rewriting
//! the PyTorch model to Burn is complex. Ideally this should avoid using
//! burn-onnx and instead code it directly in Rust.
//!
//! The onnx files should be in the `models` directory.
//!
//! This can be automated by using `workshop run insightface -- scrfd2onnx`.
//!
//! This uses the Canonical's workshop tool to launch a container with the
//! necessary dependencies to run the script and places it directly in
//! the `models` directory.
//!
//! # Notes
//!
//! It is recommended to compile in release mode to avoid stack overflow
//! issues.

use burn::prelude::*;

mod models;
pub use models::*;

pub const STRIDE: [usize; 3] = [8, 16, 32];

pub const NUM_ANCHORS: usize = 2;

/// The type of SCRFD model.
#[derive(Debug)]
pub enum ModelType {
    Scrfd1g,
    Scrfd2_5g,
    Scrfd2_5gKps,
    Scrfd10g,
    Scrfd10gKps,
    Scrfd34g,
    Scrfd500m,
    Scrfd500mKps,
}

/// The SCRFD model.
pub enum Model<B: Backend> {
    Scrfd1g(Scrfd1g<B>),
    Scrfd2_5g(Scrfd2_5g<B>),
    Scrfd2_5gKps(Scrfd2_5gKps<B>),
    Scrfd10g(Scrfd10g<B>),
    Scrfd10gKps(Scrfd10gKps<B>),
    Scrfd34g(Scrfd34g<B>),
    Scrfd500m(Scrfd500m<B>),
    Scrfd500mKps(Scrfd500mKps<B>),
}

impl<B: Backend> Model<B> {
    #[cfg(feature = "embedded")]
    pub fn from_embedded(kind: ModelType, device: &B::Device) -> Self {
        match kind {
            ModelType::Scrfd1g => Self::Scrfd1g(Scrfd1g::<B>::from_embedded(device)),
            ModelType::Scrfd2_5g => Self::Scrfd2_5g(Scrfd2_5g::<B>::from_embedded(device)),
            ModelType::Scrfd2_5gKps => Self::Scrfd2_5gKps(Scrfd2_5gKps::<B>::from_embedded(device)),
            ModelType::Scrfd10g => Self::Scrfd10g(Scrfd10g::<B>::from_embedded(device)),
            ModelType::Scrfd10gKps => Self::Scrfd10gKps(Scrfd10gKps::<B>::from_embedded(device)),
            ModelType::Scrfd34g => Self::Scrfd34g(Scrfd34g::<B>::from_embedded(device)),
            ModelType::Scrfd500m => Self::Scrfd500m(Scrfd500m::<B>::from_embedded(device)),
            ModelType::Scrfd500mKps => Self::Scrfd500mKps(Scrfd500mKps::<B>::from_embedded(device)),
        }
    }

    pub fn is_kps(&self) -> bool {
        matches!(
            self,
            Model::Scrfd2_5gKps(_) | Model::Scrfd10gKps(_) | Model::Scrfd500mKps(_)
        )
    }
}

impl<B: Backend> Model<B> {
    pub fn forward(&self, image: Tensor<B, 4>) -> Vec<Tensor<B, 3>> {
        match self {
            Model::Scrfd1g(model) => {
                let (s8, s16, s32, b8, b16, b32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32]
            }
            Model::Scrfd2_5g(model) => {
                let (s8, s16, s32, b8, b16, b32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32]
            }
            Model::Scrfd10g(model) => {
                let (s8, s16, s32, b8, b16, b32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32]
            }
            Model::Scrfd34g(model) => {
                let (s8, s16, s32, b8, b16, b32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32]
            }
            Model::Scrfd500m(model) => {
                let (s8, s16, s32, b8, b16, b32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32]
            }
            Model::Scrfd2_5gKps(model) => {
                let (s8, s16, s32, b8, b16, b32, k8, k16, k32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32, k8, k16, k32]
            }
            Model::Scrfd10gKps(model) => {
                let (s8, s16, s32, b8, b16, b32, k8, k16, k32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32, k8, k16, k32]
            }
            Model::Scrfd500mKps(model) => {
                let (s8, s16, s32, b8, b16, b32, k8, k16, k32) = model.forward(image);
                vec![s8, s16, s32, b8, b16, b32, k8, k16, k32]
            }
        }
    }
}

impl<B: Backend> Model<B> {
    pub fn detect(
        &self,
        image: Tensor<B, 4>,
        score_threshold: f32,
        nms_threshold: f32,
    ) -> Vec<Face> {
        let [_, _, h, w] = image.dims();
        let outputs = self.forward(image);

        let mut results = Vec::new();
        if self.is_kps() {
            let [s8, s16, s32, b8, b16, b32, k8, k16, k32] = outputs.try_into().unwrap();

            for (i, ((s, b), k)) in [s8, s16, s32]
                .into_iter()
                .zip([b8, b16, b32].into_iter())
                .zip([k8, k16, k32].into_iter())
                .enumerate()
            {
                results.extend(decode_stride(
                    s,
                    b,
                    Some(k),
                    STRIDE[i],
                    w,
                    h,
                    score_threshold,
                ));
            }
        } else {
            let [s8, s16, s32, b8, b16, b32] = outputs.try_into().unwrap();

            for (i, (s, b)) in [s8, s16, s32]
                .into_iter()
                .zip([b8, b16, b32].into_iter())
                .enumerate()
            {
                results.extend(decode_stride(s, b, None, STRIDE[i], w, h, score_threshold));
            }
        }
        nms(results, nms_threshold)
    }
}

#[cfg(feature = "image")]
impl<B: Backend> Model<B> {
    pub fn detect_image(
        &self,
        image: image::DynamicImage,
        score_threshold: f32,
        nms_threshold: f32,
        device: &B::Device,
    ) -> Vec<Face> {
        const INPUT_W: u32 = 640;
        const INPUT_H: u32 = 640;

        let img = image.into_rgb8();
        let (img, lb) = Self::letterbox(&img, INPUT_W, INPUT_H);
        let img = Self::image_to_tensor(&img, device);
        let mut results = self.detect(img, score_threshold, nms_threshold);
        for result in &mut results {
            result.unletterbox(&lb);
        }
        results
    }

    /// Resize keeping aspect ratio, pad to (target_w, target_h).
    /// Returns the padded RGB image plus the transform info.
    fn letterbox(
        img: &image::RgbImage,
        target_w: u32,
        target_h: u32,
    ) -> (image::RgbImage, LetterboxInfo) {
        let (w, h) = (img.width() as f32, img.height() as f32);
        let scale = (target_w as f32 / w).min(target_h as f32 / h);
        let new_w = (w * scale).round() as u32;
        let new_h = (h * scale).round() as u32;

        let resized =
            image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

        let pad_x = (target_w - new_w) / 2;
        let pad_y = (target_h - new_h) / 2;

        let mut canvas = image::RgbImage::new(target_w, target_h);
        image::imageops::overlay(&mut canvas, &resized, pad_x as i64, pad_y as i64);

        (
            canvas,
            LetterboxInfo {
                scale,
                pad_x: pad_x as f32,
                pad_y: pad_y as f32,
            },
        )
    }

    /// HWC u8 image -> normalized NCHW tensor (RGB, (x-127.5)/128).
    fn image_to_tensor(img: &image::RgbImage, device: &B::Device) -> Tensor<B, 4> {
        let (w, h) = (img.width() as usize, img.height() as usize);
        let data = img.as_raw().iter().map(|&b| b as f32).collect::<Vec<_>>();
        Tensor::<B, 1>::from_floats(data.as_slice(), device)
            .reshape([1, h, w, 3])
            .permute([0, 3, 1, 2]) // NHWC -> NCHW
            .sub_scalar(127.5f32)
            .div_scalar(128.0f32)
    }
}

#[derive(Debug, Clone)]
pub struct Face {
    pub score: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
    pub landmarks: Option<[[f32; 2]; 5]>,
}

impl Face {
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    pub fn unletterbox(&mut self, lb: &LetterboxInfo) {
        let (x1, y1) = lb.unletterbox(self.x1, self.y1);
        let (x2, y2) = lb.unletterbox(self.x2, self.y2);

        self.x1 = x1;
        self.y1 = y1;
        self.x2 = x2;
        self.y2 = y2;

        if let Some(landmarks) = &mut self.landmarks {
            for landmark in landmarks.iter_mut() {
                let (x, y) = lb.unletterbox(landmark[0], landmark[1]);
                landmark[0] = x;
                landmark[1] = y;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct LetterboxInfo {
    pub scale: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

impl LetterboxInfo {
    pub fn unletterbox(&self, x: f32, y: f32) -> (f32, f32) {
        let new_x = (x - self.pad_x) / self.scale;
        let new_y = (y - self.pad_y) / self.scale;
        (new_x, new_y)
    }
}

fn anchor_centers(stride: usize, input_w: usize, input_h: usize) -> Vec<(f32, f32)> {
    let gw = input_w / stride;
    let gh = input_h / stride;

    let mut centers = Vec::with_capacity(gw * gh * NUM_ANCHORS);
    for y in 0..gh {
        for x in 0..gw {
            let cx = (x * stride) as f32;
            let cy = (y * stride) as f32;
            for _ in 0..NUM_ANCHORS {
                centers.push((cx, cy));
            }
        }
    }
    centers
}

fn decode_stride<B: Backend>(
    scores: Tensor<B, 3>,
    bbox: Tensor<B, 3>,
    kps: Option<Tensor<B, 3>>,
    stride: usize,
    input_w: usize,
    input_h: usize,
    score_threshold: f32,
) -> Vec<Face> {
    let n = scores.dims()[1];
    let scores: Vec<f32> = scores.into_data().to_vec().unwrap();
    let bbox: Vec<f32> = bbox.into_data().to_vec().unwrap();
    let kps: Option<Vec<f32>> = kps.map(|v| v.into_data().to_vec().unwrap());
    let centers = anchor_centers(stride, input_w, input_h);

    let mut out = Vec::new();
    for i in 0..n {
        let score = scores[i];
        if score < score_threshold {
            continue;
        };

        let (cx, cy) = centers[i];
        let l = bbox[i * 4] * stride as f32;
        let t = bbox[i * 4 + 1] * stride as f32;
        let r = bbox[i * 4 + 2] * stride as f32;
        let b = bbox[i * 4 + 3] * stride as f32;

        let landmarks = kps.as_ref().map(|kps| {
            let mut landmarks = [[0.0; 2]; 5];
            for j in 0..5 {
                let dx = kps[i * 10 + 2 * j];
                let dy = kps[i * 10 + 2 * j + 1];
                landmarks[j] = [cx + dx * stride as f32, cy + dy * stride as f32];
            }
            landmarks
        });

        out.push(Face {
            score,
            x1: cx - l,
            y1: cy - t,
            x2: cx + r,
            y2: cy + b,
            landmarks,
        });
    }
    out
}

fn iou(a: &Face, b: &Face) -> f32 {
    let xx1 = a.x1.max(b.x1);
    let yy1 = a.y1.max(b.y1);
    let xx2 = a.x2.min(b.x2);
    let yy2 = a.y2.min(b.y2);
    let w = (xx2 - xx1).max(0.0);
    let h = (yy2 - yy1).max(0.0);
    let inter = w * h;
    let area_a = (a.x2 - a.x1) * (a.y2 - a.y1);
    let area_b = (b.x2 - b.x1) * (b.y2 - b.y1);
    inter / (area_a + area_b - inter + 1e-9)
}

fn nms(mut dets: Vec<Face>, thresh: f32) -> Vec<Face> {
    dets.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());
    let mut keep: Vec<Face> = Vec::new();
    'outer: for d in dets {
        for k in &keep {
            if iou(&d, k) >= thresh {
                continue 'outer;
            }
        }
        keep.push(d);
    }
    keep
}

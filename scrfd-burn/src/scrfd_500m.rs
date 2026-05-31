use crate::{Face, LetterboxInfo};
use image::{DynamicImage, RgbImage};

const INPUT_W: usize = 640;
const INPUT_H: usize = 640;
const STRIDE: [usize; 3] = [8, 16, 32];
const NUM_ANCHORS: usize = 2;

include!(concat!(env!("OUT_DIR"), "/scrfd_500m/scrfd_500m.rs"));

#[derive(Debug)]
pub struct InferenceConfig {}

impl<B: Backend> Model<B> {
    pub fn detect_image(
        &self,
        image: DynamicImage,
        score_threshold: f32,
        nms_threshold: f32,
    ) -> Vec<Face> {
        let img = image.into_rgb8();
        let (img, lb) = letterbox(&img, INPUT_W as u32, INPUT_H as u32);
        let img = image_to_tensor(&img, &self.device);
        let res = self.detect(img, &lb, score_threshold, nms_threshold);
        res
    }

    pub fn detect(
        &self,
        input: Tensor<B, 4>,
        lb: &LetterboxInfo,
        score_threshold: f32,
        nms_threshold: f32,
    ) -> Vec<Face> {
        let (s8, s16, s32, b8, b16, b32) = self.forward(input);

        let mut results = Vec::new();
        results.extend(decode_stride(s8, b8, STRIDE[0], score_threshold));
        results.extend(decode_stride(s16, b16, STRIDE[1], score_threshold));
        results.extend(decode_stride(s32, b32, STRIDE[2], score_threshold));
        for face in &mut results {
            face.rescale_to_original(&lb);
        }
        let results = nms(results, nms_threshold);
        results
    }
}

fn anchor_centers(stride: usize) -> Vec<(f32, f32)> {
    let gw = INPUT_W / stride;
    let gh = INPUT_H / stride;

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

/// Resize keeping aspect ratio, pad to (target_w, target_h).
/// Returns the padded RGB image plus the transform info.
fn letterbox(img: &RgbImage, target_w: u32, target_h: u32) -> (RgbImage, LetterboxInfo) {
    let (w, h) = (img.width() as f32, img.height() as f32);
    let scale = (target_w as f32 / w).min(target_h as f32 / h);
    let new_w = (w * scale).round() as u32;
    let new_h = (h * scale).round() as u32;

    let resized = image::imageops::resize(img, new_w, new_h, image::imageops::FilterType::Triangle);

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
fn image_to_tensor<B: Backend>(img: &image::RgbImage, device: &B::Device) -> Tensor<B, 4> {
    let (w, h) = (img.width() as usize, img.height() as usize);
    let data = img.as_raw().iter().map(|&b| b as f32).collect::<Vec<_>>();
    Tensor::<B, 1>::from_floats(data.as_slice(), device)
        .reshape([1, h, w, 3])
        .permute([0, 3, 1, 2]) // NHWC -> NCHW
        .sub_scalar(127.5f32)
        .div_scalar(128.0f32)
}

fn decode_stride<B: Backend>(
    scores: Tensor<B, 3>,
    bbox: Tensor<B, 3>,
    stride: usize,
    score_threshold: f32,
) -> Vec<Face> {
    let n = scores.dims()[1];
    let scores: Vec<f32> = scores.into_data().to_vec().unwrap();
    let bbox: Vec<f32> = bbox.into_data().to_vec().unwrap();
    let centers = anchor_centers(stride);

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
        out.push(Face {
            x1: cx - l,
            y1: cy - t,
            x2: cx + r,
            y2: cy + b,
            score,
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

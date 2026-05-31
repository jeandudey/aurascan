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

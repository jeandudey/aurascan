use burn::backend::Rocm;
use burn::backend::rocm::RocmDevice;
use image::{Rgb, RgbImage};
use imageproc::drawing::{draw_filled_circle_mut, draw_hollow_rect_mut};
use imageproc::rect::Rect;
use scrfd_burn::{Face, Model, ModelType};

fn main() {
    let image_path = std::env::args().nth(1).unwrap();
    let orig_image = image::open(&image_path).unwrap();

    let device = RocmDevice::default();
    let model = Model::<Rocm>::from_embedded(ModelType::Scrfd500mKps, &device);

    let faces = model.detect_image(orig_image.clone(), 0.3, 0.4, &device);
    let mut result = orig_image.into_rgb8();
    draw_detections(&mut result, &faces);
    result.save("output.png").unwrap();
}

/// Distinct colors cycled per detection.
const PALETTE: &[Rgb<u8>] = &[
    Rgb([255, 0, 0]),   // red
    Rgb([0, 255, 0]),   // green
    Rgb([0, 128, 255]), // blue
    Rgb([255, 255, 0]), // yellow
    Rgb([255, 0, 255]), // magenta
    Rgb([0, 255, 255]), // cyan
    Rgb([255, 128, 0]), // orange
];

/// Draw each detection as a hollow rectangle in a cycling color.
/// `dets` are in original-image pixel coords (post rescale_to_original).
fn draw_detections(img: &mut RgbImage, dets: &[Face]) {
    let (iw, ih) = (img.width() as i32, img.height() as i32);
    for (i, d) in dets.iter().enumerate() {
        let color = PALETTE[i % PALETTE.len()];

        // clamp to image bounds
        let x1 = (d.x1.round() as i32).clamp(0, iw - 1);
        let y1 = (d.y1.round() as i32).clamp(0, ih - 1);
        let x2 = (d.x2.round() as i32).clamp(0, iw - 1);
        let y2 = (d.y2.round() as i32).clamp(0, ih - 1);
        let w = (x2 - x1).max(1);
        let h = (y2 - y1).max(1);

        // draw a few nested rects for thicker lines
        for t in 0..2 {
            let r = Rect::at(x1 + t, y1 + t)
                .of_size((w - 2 * t).max(1) as u32, (h - 2 * t).max(1) as u32);
            draw_hollow_rect_mut(img, r, color);
        }

        if let Some(landmarks) = d.landmarks {
            for [lx, ly] in landmarks {
                let cx = (lx.round() as i32).clamp(0, iw - 1);
                let cy = (ly.round() as i32).clamp(0, ih - 1);
                draw_filled_circle_mut(img, (cx, cy), 2, color);
            }
        }
    }
}

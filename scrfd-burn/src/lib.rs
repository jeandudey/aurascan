//! # SCRFD model.
//!
//! This currently is implemented using burn-onnx to save time as rewriting
//! the PyTorch model to Burn is complex. Ideally this should avoid using
//! burn-onnx and instead code it directly in Rust.
//!
//! The `scrfd_500m.onnx` file should be in the `models` directory.
//!
//! This can be automated by using `workshop run insightface -- scrfd2onnx`.
//!
//! This uses the Canonical's workshop tool to launch a container with the
//! necessary dependencies to run the script and places it directly in
//! the `models` directory.

pub mod scrfd_500m;

#[derive(Debug, Clone)]
pub struct Face {
    pub score: f32,
    pub x1: f32,
    pub y1: f32,
    pub x2: f32,
    pub y2: f32,
}

impl Face {
    pub fn width(&self) -> f32 {
        self.x2 - self.x1
    }

    pub fn height(&self) -> f32 {
        self.y2 - self.y1
    }

    fn rescale_to_original(&mut self, lb: &LetterboxInfo) {
        self.x1 = (self.x1 - lb.pad_x) / lb.scale;
        self.y1 = (self.y1 - lb.pad_y) / lb.scale;
        self.x2 = (self.x2 - lb.pad_x) / lb.scale;
        self.y2 = (self.y2 - lb.pad_y) / lb.scale;
    }
}

#[derive(Debug, Clone)]
pub struct LetterboxInfo {
    pub scale: f32,
    pub pad_x: f32,
    pub pad_y: f32,
}

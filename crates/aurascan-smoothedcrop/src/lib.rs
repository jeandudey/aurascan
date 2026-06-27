#[derive(Debug)]
pub struct SmoothedCrop {
    center_x: f32,
    center_y: f32,
    side: f32,
    initialized: bool,
}

impl SmoothedCrop {
    /// Create a new [`SmoothedCrop`].
    pub fn new() -> Self {
        Self {
            center_x: 0.0,
            center_y: 0.0,
            side: 0.0,
            initialized: false,
        }
    }

    /// Advances the smoothed crop with a new bounding box.
    pub fn advance(&mut self, region: Rect, settings: &Settings) {
        let x = region.x as f32;
        let y = region.y as f32;
        let w = region.width as f32;
        let h = region.height as f32;

        let expanded_left = x - w * settings.expansion_side;
        let expanded_right = x + w + w * settings.expansion_side;
        let expanded_top = y - h * settings.expansion_top;
        let expanded_bottom = y + h + h * settings.expansion_bottom;

        let expanded_width = expanded_right - expanded_left;
        let expanded_height = expanded_bottom - expanded_top;
        let expanded_side = expanded_width.max(expanded_height);

        let expanded_center_x = (expanded_left + expanded_right) / 2.0;
        let expanded_center_y = (expanded_top + expanded_bottom) / 2.0;

        if !self.initialized {
            self.center_x = expanded_center_x;
            self.center_y = expanded_center_y;
            self.side = expanded_side;
            self.initialized = true;
        } else {
            self.center_x += settings.alpha * (expanded_center_x - self.center_x);
            self.center_y += settings.alpha * (expanded_center_y - self.center_y);
            self.side += settings.alpha * (expanded_side - self.side);
        }
    }

    /// Returns the rect of the cropped region, if not yet initialized with data
    /// returns the full frame.
    pub fn rect(&self, frame_w: u32, frame_h: u32) -> Rect {
        if !self.initialized {
            return Rect {
                x: 0,
                y: 0,
                width: frame_w,
                height: frame_h,
            };
        }

        let side = self
            .side
            .min((frame_w - 1) as f32)
            .min((frame_h - 1) as f32)
            .max(1.0);

        let mut left = self.center_x - (side / 2.0);
        let mut top = self.center_y - (side / 2.0);
        let mut right = left + side;
        let mut bottom = top + side;

        if left < 0.0 {
            right -= left;
            left = 0.0;
        }

        if top < 0.0 {
            bottom -= top;
            top = 0.0;
        }

        if right > frame_w as f32 {
            left -= right - frame_w as f32;
            right = frame_w as f32;
        }

        if bottom > frame_h as f32 {
            top -= bottom - frame_h as f32;
            bottom = frame_h as f32;
        }

        let left = left.max(0.0);
        let top = top.max(0.0);
        let right = right.min(frame_w as f32);
        let bottom = bottom.min(frame_h as f32);

        Rect {
            x: left.round() as u32,
            y: top.round() as u32,
            width: (right - left).round().max(0.0) as u32,
            height: (bottom - top).round().max(0.0) as u32,
        }
    }
}

impl Default for SmoothedCrop {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug)]
pub struct Settings {
    pub expansion_top: f32,
    pub expansion_bottom: f32,
    pub expansion_side: f32,
    pub alpha: f32,
}

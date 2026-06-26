#[derive(Debug)]
pub struct LowPassFilter {
    initialized: bool,
    prev_y: f64,
}

impl LowPassFilter {
    pub fn new() -> Self {
        Self {
            initialized: false,
            prev_y: 0.0,
        }
    }

    pub fn filter(&mut self, x: f64, alpha: f64) -> f64 {
        debug_assert!(alpha > 0.0 && alpha <= 1.0);

        let hat = if self.initialized {
            alpha * x + (1.0 - alpha) * self.prev_y
        } else {
            self.initialized = true;
            x
        };
        self.prev_y = hat;
        hat
    }

    pub fn last(&self) -> f64 {
        self.prev_y
    }
}

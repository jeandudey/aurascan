use std::f64;

use crate::LowPassFilter;

#[derive(Debug)]
pub struct OneEuroFilter {
    min_cutoff: f64,
    beta: f64,
    d_cutoff: f64,
    x: LowPassFilter,
    dx: LowPassFilter,
    last_time: Option<f64>,
    initialized: bool,
}

impl OneEuroFilter {
    pub fn new(min_cutoff: f64, beta: f64, d_cutoff: f64) -> Self {
        Self {
            min_cutoff,
            beta,
            d_cutoff,
            x: LowPassFilter::new(),
            dx: LowPassFilter::new(),
            last_time: None,
            initialized: false,
        }
    }

    pub fn filter(&mut self, value: f64, t: f64) -> f64 {
        let dt = self.update_time(t);

        let dvalue = if self.initialized {
            (value - self.x.last()) / dt
        } else {
            0.0
        };

        self.initialized = true;
        let edvalue = self.dx.filter(dvalue, Self::alpha(self.d_cutoff, dt));

        let cutoff = self.min_cutoff + self.beta * edvalue.abs();

        self.x.filter(value, Self::alpha(cutoff, dt))
    }

    fn update_time(&mut self, t: f64) -> f64 {
        let dt = match self.last_time {
            Some(prev) if t > prev => t - prev,
            _ => 1.0 / 60.0,
        };
        self.last_time = Some(t);
        dt
    }

    fn alpha(cutoff: f64, dt: f64) -> f64 {
        let tau = 1.0 / (f64::consts::TAU * cutoff);
        1.0 / (1.0 + tau / dt)
    }
}

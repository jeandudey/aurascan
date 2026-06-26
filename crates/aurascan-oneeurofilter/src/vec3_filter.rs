use crate::OneEuroFilter;

#[derive(Debug)]
pub struct Vec3Filter {
    x: OneEuroFilter,
    y: OneEuroFilter,
    z: OneEuroFilter,
}

impl Vec3Filter {
    pub fn new(min_cutoff: f64, beta: f64, d_cutoff: f64) -> Self {
        Self {
            x: OneEuroFilter::new(min_cutoff, beta, d_cutoff),
            y: OneEuroFilter::new(min_cutoff, beta, d_cutoff),
            z: OneEuroFilter::new(min_cutoff, beta, d_cutoff),
        }
    }

    pub fn filter(&mut self, v: [f64; 3], t: f64) -> [f64; 3] {
        [
            self.x.filter(v[0], t),
            self.y.filter(v[1], t),
            self.z.filter(v[2], t),
        ]
    }
}

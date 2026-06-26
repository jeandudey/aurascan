use crate::{QuatFilter, Vec3Filter};

#[derive(Debug)]
pub struct HeadPoseFilter {
    translation: Vec3Filter,
    rotation: QuatFilter,
}

impl HeadPoseFilter {
    pub fn new(min_cutoff: f64, beta: f64, d_cutoff: f64) -> Self {
        Self {
            translation: Vec3Filter::new(min_cutoff, beta, d_cutoff),
            rotation: QuatFilter::new(min_cutoff, beta, d_cutoff),
        }
    }

    pub fn filter(
        &mut self,
        translation: [f64; 3],
        rotation: [f64; 4],
        t: f64,
    ) -> ([f64; 3], [f64; 4]) {
        (
            self.translation.filter(translation, t),
            self.rotation.filter(rotation, t),
        )
    }
}

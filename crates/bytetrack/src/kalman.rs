use crate::{Covariance, Measurement, State};
use nalgebra::SMatrix;
use std::sync::LazyLock;

const STD_WEIGHT_POSITION: f32 = 1.0 / 20.0;
const STD_WEIGHT_VELOCITY: f32 = 1.0 / 160.0;

#[derive(Debug)]
struct Matrices {
    motion_mat: SMatrix<f32, 8, 8>,
    update_mat: SMatrix<f32, 4, 8>,
}

impl Matrices {
    pub fn new() -> Self {
        let mut motion_mat = SMatrix::<f32, 8, 8>::identity();
        for i in 0..4 {
            motion_mat[(i, i + 4)] = 1.0;
        }

        let update_mat = SMatrix::<f32, 4, 8>::identity();

        Self {
            motion_mat,
            update_mat,
        }
    }
}

static MATRICES: LazyLock<Matrices> = LazyLock::new(|| Matrices::new());

pub fn initiate(measurement: &Measurement) -> (State, Covariance) {
    let mut mean = State::zeros();
    mean.fixed_rows_mut::<4>(0).copy_from(measurement);
    let std = [
        2.0 * STD_WEIGHT_POSITION * measurement[3],
        2.0 * STD_WEIGHT_POSITION * measurement[3],
        1e-2,
        2.0 * STD_WEIGHT_POSITION * measurement[3],
        10.0 * STD_WEIGHT_VELOCITY * measurement[3],
        10.0 * STD_WEIGHT_VELOCITY * measurement[3],
        1e-4,
        10.0 * STD_WEIGHT_VELOCITY * measurement[3],
    ];
    let covariance =
        Covariance::from_diagonal(&State::from_iterator(std.into_iter().map(|s| s * s)));
    (mean, covariance)
}

pub fn predict(mean: &mut State, covariance: &mut Covariance) {
    let std_pos = [
        STD_WEIGHT_POSITION * mean[3],
        STD_WEIGHT_POSITION * mean[3],
        1e-2,
        STD_WEIGHT_POSITION * mean[3],
    ];
    let std_vel = [
        STD_WEIGHT_VELOCITY * mean[3],
        STD_WEIGHT_VELOCITY * mean[3],
        1e-5,
        STD_WEIGHT_VELOCITY * mean[3],
    ];
    let motion_cov = Covariance::from_diagonal(&State::from_iterator(
        std_pos
            .into_iter()
            .chain(std_vel.into_iter())
            .map(|s| s * s),
    ));

    *mean = MATRICES.motion_mat * *mean;
    *covariance = MATRICES.motion_mat * *covariance * MATRICES.motion_mat.transpose() + motion_cov;
}

pub fn update(mean: &mut State, covariance: &mut Covariance, measurement: &Measurement) {
    let std = [
        STD_WEIGHT_POSITION * mean[3],
        STD_WEIGHT_POSITION * mean[3],
        1e-1,
        STD_WEIGHT_POSITION * mean[3],
    ];
    let innovation_cov = SMatrix::<f32, 4, 4>::from_diagonal(&Measurement::from_iterator(
        std.into_iter().map(|s| s * s),
    ));

    let proj_mean = MATRICES.update_mat * (*mean);
    let proj_cov =
        MATRICES.update_mat * (*covariance) * MATRICES.update_mat.transpose() + innovation_cov;
    let proj_inv = proj_cov
        .try_inverse()
        .expect("projection covariance singular");
    let gain = *covariance * MATRICES.update_mat.transpose() * proj_inv;
    let innovation = measurement - proj_mean;
    *mean = *mean + gain * innovation;
    *covariance = (Covariance::identity() - gain * MATRICES.update_mat) * (*covariance);
}

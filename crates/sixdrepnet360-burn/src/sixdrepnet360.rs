use crate::block::{LayerBlock, LayerBlockConfig};
#[cfg(feature = "pretrained")]
use crate::weights;
use burn::nn::{
    BatchNorm, BatchNormConfig, Linear, LinearConfig, PaddingConfig2d, Relu,
    conv::{Conv2d, Conv2dConfig},
    pool::{AvgPool2d, AvgPool2dConfig, MaxPool2d, MaxPool2dConfig},
};
use burn::prelude::*;
use burn_store::{ModuleSnapshot, PytorchStore, PytorchStoreError};
use std::path::PathBuf;

/// 6DRepNet360 model.
#[derive(Debug, Module)]
pub struct SixDRepNet360<B: Backend> {
    conv1: Conv2d<B>,
    bn1: BatchNorm<B>,
    relu: Relu,
    maxpool: MaxPool2d,
    layer1: LayerBlock<B>,
    layer2: LayerBlock<B>,
    layer3: LayerBlock<B>,
    layer4: LayerBlock<B>,
    avgpool: AvgPool2d,
    linear_reg: Linear<B>,
}

impl<B: Backend> SixDRepNet360<B> {
    pub fn new(layers: [usize; 4], device: &B::Device) -> Self {
        SixDRepNet360Config::new(layers).init(device)
    }

    pub fn forward(&self, input: Tensor<B, 4>) -> Tensor<B, 3> {
        let out = self.conv1.forward(input);
        let out = self.bn1.forward(out);
        let out = self.relu.forward(out);
        let out = self.maxpool.forward(out);

        let out = self.layer1.forward(out);
        let out = self.layer2.forward(out);
        let out = self.layer3.forward(out);
        let out = self.layer4.forward(out);

        let out = self.avgpool.forward(out);
        let [b, c, h, w] = out.dims();
        let out = out.reshape([b, c * h * w]);

        let out = self.linear_reg.forward(out);
        compute_rotation_matrix_from_ortho6d(out)
    }

    pub fn detect(&self, input: Tensor<B, 4>) -> Vec<[f32; 3]> {
        let euler = compute_euler_angles_from_rotation_matrices(self.forward(input))
            .mul_scalar(180.0 / std::f32::consts::PI);

        let batch = euler.dims()[0];
        let flat: Vec<f32> = euler.into_data().to_vec().unwrap();
        (0..batch)
            .map(|i| [flat[i * 3], flat[i * 3 + 1], flat[i * 3 + 2]])
            .collect()
    }
}

impl<B: Backend> SixDRepNet360<B> {
    pub fn from_file(
        torch_weights: impl Into<PathBuf>,
        device: &B::Device,
    ) -> Result<Self, PytorchStoreError> {
        let mut model = Self::new([3, 4, 6, 3], device);
        Self::load_weights(&mut model, torch_weights)?;
        Ok(model)
    }

    pub fn load_weights(
        model: &mut Self,
        torch_weights: impl Into<PathBuf>,
    ) -> Result<(), PytorchStoreError> {
        let mut store = PytorchStore::from_file(torch_weights.into())
            // Map *.downsample.0.* -> *.downsample.conv.*
            .with_key_remapping("(.+)\\.downsample\\.0\\.(.+)", "$1.downsample.conv.$2")
            // Map *.downsample.1.* -> *.downsample.bn.*
            .with_key_remapping("(.+)\\.downsample\\.1\\.(.+)", "$1.downsample.bn.$2")
            // Map layer[i].[j].* -> layer[i].blocks.[j].*
            .with_key_remapping("(layer[1-4])\\.([0-9]+)\\.(.+)", "$1.blocks.$2.$3");
        model.load_from(&mut store)?;
        Ok(())
    }
}

#[cfg(feature = "pretrained")]
impl<B: Backend> SixDRepNet360<B> {
    /// Download a pretrained 6DRepNet360 model from a PyTorch weights file.
    #[cfg(feature = "pretrained")]
    pub fn pretrained(device: &B::Device) -> Result<Self, PytorchStoreError> {
        let mut model = Self::new([3, 4, 6, 3], device);
        Self::download_weights(&mut model)?;
        Ok(model)
    }

    /// Download the pretrained weights for the model.
    pub fn download_weights(model: &mut Self) -> Result<(), PytorchStoreError> {
        let torch_weights = weights::download().map_err(|err| {
            PytorchStoreError::Other(format!("Could not download weights.\nError: {err}"))
        })?;
        Self::load_weights(model, torch_weights)
    }
}

/// Configuration for the 6DRepNet360 model.
#[derive(Debug)]
pub struct SixDRepNet360Config {
    pub layers: [usize; 4],
}

impl SixDRepNet360Config {
    /// Create a new configuration with the given layer sizes.
    pub fn new(layers: [usize; 4]) -> Self {
        Self { layers }
    }

    /// Initialize the model with the given device.
    pub fn init<B: Backend>(&self, device: &B::Device) -> SixDRepNet360<B> {
        const EXPANSION: usize = 4;

        SixDRepNet360 {
            conv1: Conv2dConfig::new([3, 64], [7, 7])
                .with_stride([2, 2])
                .with_padding(PaddingConfig2d::Explicit(3, 3, 3, 3))
                .with_bias(false)
                .init(device),
            bn1: BatchNormConfig::new(64).init(device),
            relu: Relu::new(),
            maxpool: MaxPool2dConfig::new([3, 3])
                .with_strides([2, 2])
                .with_padding(PaddingConfig2d::Explicit(1, 1, 1, 1))
                .init(),
            layer1: LayerBlockConfig::new(self.layers[0], 64, 64 * EXPANSION, 1, true).init(device),
            layer2: LayerBlockConfig::new(self.layers[1], 64 * EXPANSION, 128 * EXPANSION, 2, true)
                .init(device),
            layer3: LayerBlockConfig::new(
                self.layers[2],
                128 * EXPANSION,
                256 * EXPANSION,
                2,
                true,
            )
            .init(device),
            layer4: LayerBlockConfig::new(
                self.layers[3],
                256 * EXPANSION,
                512 * EXPANSION,
                2,
                true,
            )
            .init(device),
            avgpool: AvgPool2dConfig::new([7, 7]).init(),
            linear_reg: LinearConfig::new(512 * EXPANSION, 6).init(device),
        }
    }
}

pub fn normalize_vector<B: Backend>(v: Tensor<B, 2>) -> Tensor<B, 2> {
    let v_mag = v.clone().powf_scalar(2.0).sum_dim(1).sqrt(); // [B, 1]
    let v_mag = v_mag.clamp_min(1e-8).expand(v.dims());
    v / v_mag
}

pub fn cross_product<B: Backend>(u: Tensor<B, 2>, v: Tensor<B, 2>) -> Tensor<B, 2> {
    let u0 = u.clone().narrow(1, 0, 1);
    let u1 = u.clone().narrow(1, 1, 1);
    let u2 = u.narrow(1, 2, 1);
    let v0 = v.clone().narrow(1, 0, 1);
    let v1 = v.clone().narrow(1, 1, 1);
    let v2 = v.narrow(1, 2, 1);

    let i = u1.clone() * v2.clone() - u2.clone() * v1.clone();
    let j = u2 * v0.clone() - u0.clone() * v2;
    let k = u0 * v1 - u1 * v0;
    Tensor::cat(vec![i, j, k], 1)
}

pub fn compute_rotation_matrix_from_ortho6d<B: Backend>(poses: Tensor<B, 2>) -> Tensor<B, 3> {
    let batch = poses.dims()[0];

    let x_raw = poses.clone().narrow(1, 0, 3); // [batch, 3]
    let y_raw = poses.narrow(1, 3, 3); // [batch, 3]

    let x = normalize_vector(x_raw);
    let z = normalize_vector(cross_product(x.clone(), y_raw));
    let y = cross_product(z.clone(), x.clone());

    let x = x.reshape([batch, 3, 1]);
    let y = y.reshape([batch, 3, 1]);
    let z = z.reshape([batch, 3, 1]);
    Tensor::cat(vec![x, y, z], 2) // [batch, 3, 3]
}

pub fn compute_euler_angles_from_rotation_matrices<B: Backend>(r: Tensor<B, 3>) -> Tensor<B, 2> {
    let batch = r.dims()[0];

    // Element accessor: slice [batch, i, j] -> [batch, 1]
    let e = |i: usize, j: usize| {
        r.clone()
            .slice([0..batch, i..i + 1, j..j + 1])
            .reshape([batch, 1])
    };

    let r00 = e(0, 0);
    let r10 = e(1, 0);
    let r11 = e(1, 1);
    let r12 = e(1, 2);
    let r20 = e(2, 0);
    let r21 = e(2, 1);
    let r22 = e(2, 2);

    // sy = sqrt(R00^2 + R10^2)
    let sy = (r00.clone().powf_scalar(2.0) + r10.clone().powf_scalar(2.0)).sqrt();

    // singular = (sy < 1e-6) as float  -> 1.0 where singular, else 0.0
    let singular = sy.clone().lower_elem(1e-6).float();
    let non_singular = singular.clone().neg().add_scalar(1.0); // 1 - singular

    // Non-singular branch
    let x = r21.atan2(r22.clone());
    let y = r20.clone().neg().atan2(sy.clone());
    let z = r10.clone().atan2(r00);

    // Singular branch
    let xs = r12.neg().atan2(r11);
    let ys = r20.neg().atan2(sy);
    let zs = r10.mul_scalar(0.0); // exactly zero, shape-preserving

    // Blend
    let out_x = x * non_singular.clone() + xs * singular.clone();
    let out_y = y * non_singular.clone() + ys * singular.clone();
    let out_z = z * non_singular + zs * singular;

    Tensor::cat(vec![out_x, out_y, out_z], 1) // [batch, 3]
}

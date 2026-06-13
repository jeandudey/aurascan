// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # 6DRepNet360 implementation using Burn.
//!
//! This crate provides a [`burn`] implementation of the [6DRepNet360] model
//! specified in ["Towards Robust and Unconstrained Full Range of Rotation Head Pose Estimation"](https://ieeexplore.ieee.org/document/10477888), IEEE
//! Transactions on Image Processing 2024.
//!
//! It currently does not provide training support, but only inference, although
//! this is planned. Contributions are welcome!
//!
//! # Usage
//!
//! In your `Cargo.toml`:
//!
//! ```toml
//! sixdrepnet360-burn = { git = "https://github.com/jeandudey/aurascan" }
//! ```
//! # Features
//!
//! - `pretrained`: Adds the ability to download the pre-trained weights.
//!
//! # Citing
//!
//! As this crate is only a port of the original 6DRepNet360 implementation,
//! please cite the original paper if you use this crate and/or find their
//! work useful:
//!
//! ```bibtex
//! @ARTICLE{10477888,
//!  author={Hempel, Thorsten and Abdelrahman, Ahmed A. and Al-Hamadi, Ayoub},
//!  journal={IEEE Transactions on Image Processing},
//!  title={Toward Robust and Unconstrained Full Range of Rotation Head Pose Estimation},
//!  year={2024},
//!  volume={33},
//!  number={},
//!  pages={2377-2387},
//!  doi={10.1109/TIP.2024.3378180}}
//! ```
//!
//! # License
//!
//! This crate is licensed under the MIT license (same license as the original
//! [6DRepNet360] implementation) or the Apache-2.0 license.
//!
//! Weights are not included in the crate, but can be downloaded from the
//! original [6DRepNet360] repository. Or downloaded using the `pretrained`
//! feature which will download the weights from the author's link listed in
//! the [6DRepNet360] README.md file. Using the `pretrained` feature is not
//! recommended for production usage.
//!
//! Ideally you should train the model on your own dataset as the pre-trained
//! weights are not clear if they are suitable for commercial use as the
//! license of the original 300W-LP dataset specifically states that it is not
//! for commercial use. Moreover, the authors do not specify the license
//! explicitly of their pre-trained weights, so we err on the side of caution
//! and assume the license is not clear.
//!
//! So, using the `pretrained` feature is highly likely not suitable for commercial
//! usage, and using it makes the crate non-open-source.
//!
//! [6DRepNet360]: https://github.com/thohemp/6DRepNet360

mod block;
pub mod sixdrepnet360;
#[cfg(feature = "pretrained")]
pub mod weights;

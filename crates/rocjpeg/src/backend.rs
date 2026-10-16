// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

/// Decoder backend.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backend {
    /// Use the VCN hardware JPEG engine.
    #[default]
    Hardware,
    /// Hybrid CPU/GPU decoding.
    Hybrid,
}

impl From<Backend> for rocjpeg_sys::RocJpegBackend {
    fn from(backend: Backend) -> Self {
        match backend {
            Backend::Hardware => Self::ROCJPEG_BACKEND_HARDWARE,
            Backend::Hybrid => Self::ROCJPEG_BACKEND_HYBRID,
        }
    }
}

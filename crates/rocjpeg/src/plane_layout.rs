// SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
// SPDX-License-Identifier: MIT

/// Size of one plane of a decoded image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaneLayout {
    /// Bytes per row without padding. The pitch passed to [`Image::set_plane`] must be at
    /// least this.
    pub row_bytes: usize,
    /// Number of rows.
    pub rows: usize,
}

impl PlaneLayout {
    /// Size of the plane in bytes when using `row_bytes` as pitch.
    pub fn size(&self) -> usize {
        self.row_bytes * self.rows
    }
}

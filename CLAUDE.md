# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

AuraScan is a real-time head pose estimation and face detection application. It combines a GTK4/Relm4 GUI with a GStreamer video pipeline that runs deep learning inference (SCRFD face detection + 6DRepNet360 pose estimation) using the Burn ML framework.

## Build Commands

```bash
# Build the full workspace
cargo build --release

# Build only the GStreamer plugin (with backend features)
cargo build --release -p gst-plugin-aurascan --features vulkan
cargo build --release -p gst-plugin-aurascan --features rocm
cargo build --release -p gst-plugin-aurascan --features "scrfd-embedded,sixdrepnet360-pretrained"

# Run the GUI application
cargo run --release -p aurascan

# Export ONNX models from Python (requires Python env setup)
cargo make --makefile models/Makefile.toml export

# Launch a test GStreamer pipeline
cargo make --makefile models/Makefile.toml launch
```

Requires Rust 1.95.0+, GStreamer 1.28+, GTK4, and libmatio system libraries.

The release profile keeps debug symbols and does not strip binaries (intentional for profiling).

## Architecture

The codebase is a Cargo workspace with four layers:

**GUI** (`app/aurascan`) — GTK4 + Libadwaita + Relm4 reactive frontend. `app.rs` holds the Relm4 component with all UI state. `pipeline.rs` constructs and manages the GStreamer pipeline. UI subcomponents (source selector, resolution selector, status bar) are each their own Relm4 component.

**GStreamer Plugin** (`crates/gst-plugin-aurascan`) — Registers custom GStreamer elements: `scrfd` (face detection inference), `scrfdtensordec` (decodes SCRFD tensor output to detection metadata), `sixdrepnet360` (head pose inference), `sixdrepnet360tensordec` (decodes to Euler angles), `bytetracker`, and `headposeinferencebin` (composite bin that links them all). Multi-backend support (Vulkan/ROCm/Flex) is gated by Cargo features. Pose data travels downstream via a custom `EulerAnglesMeta` GStreamer metadata type.

**ML Crates**:
- `crates/scrfd-burn` — SCRFD face detector. ONNX models are converted to Burn IR at build time via `burn_onnx::ModelGen`. Supports multiple model sizes (500m through 34g, with optional KPS keypoints). The `scrfd-embedded` feature bakes weights into the binary.
- `crates/sixdrepnet360-burn` — 6DRepNet360 head pose estimator. Implemented directly as a Burn architecture (not ONNX-converted). Outputs yaw/pitch/roll + 3D position. The `sixdrepnet360-pretrained` feature loads weights from disk at runtime via the `dirs` crate.

**Dataset/FFI Crates**:
- `crates/burn-3ddfa` — Utilities for the 300W-LP 3D face landmark dataset (download, extract, parse MATLAB .mat files, expose as a Burn dataset).
- `crates/matio` / `crates/matio-sys` — Safe Rust wrapper and raw FFI bindings (`bindgen`-generated) for the libmatio C library.

## Key Dependencies

- **Burn 0.21** — ML framework providing backend abstraction (Vulkan via wgpu, ROCm via CUDA/HIP, Flex for CPU)
- **GStreamer 0.25** — Video pipeline; bindings from the `gstreamer` Rust crate family
- **Relm4 0.11** — Reactive/MVVM GUI framework on top of GTK4

## Feature Flags

The GStreamer plugin's behavior is controlled by Cargo features:

| Feature | Effect |
|---|---|
| `vulkan` | Enable Vulkan/wgpu backend |
| `rocm` | Enable ROCm backend |
| `scrfd-embedded` | Embed SCRFD weights in the binary |
| `sixdrepnet360-pretrained` | Load 6DRepNet360 weights from disk at runtime |

## Linker Note

`.cargo/config.toml` sets `-Wl,--no-rosegment` to prevent segmentation issues with GStreamer on Linux. Do not remove this flag.

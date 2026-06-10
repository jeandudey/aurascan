---
name: gstreamer-plugin
description: Use when working, authoring, modifying GStreamer plugins written in Rust.
---

# GStreamer Plugin

This skill provides tools and knowledge for working with GStreamer plugins
written in Rust, especially the plugins under `crates/gst-plugin-aurascan`.

These are a set of plugins that provide video analysis and processing
capabilities for the AuraScan project, including AI inference on video frames
using the Burn machine learning framework and also the `onnxinference` element
from the upstream GStreamer project, so when working on inference plugins,
it's _really_ important to keep inference and tensor decoding decoupled so that
inference backend can be swapped out easily.

## Guidelines

- Keep inference and tensor decoding decoupled so that inference backend can be swapped out easily.
- Element names must match the GStreamer naming convention.
- Tensor decoder elements must be named `<model>tensordec`.
- Inference elements must be named `<framework>-<model>inference`, for example, `burn-<model>inference`.
- When creating new elements add a launch task to `crates/gst-plugin-aurascan/Makefile.toml` for end to end testing.
- Testing must always be performed using `--release` as Burn overflows the stack with debug builds.
- If the inference elements require weights these must not be embedded without a feature flag, and should always provide a way to load them from disk.

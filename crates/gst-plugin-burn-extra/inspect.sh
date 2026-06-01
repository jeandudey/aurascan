#!/usr/bin/env bash
cargo build --release --features embedded,vulkan || exit 1
GST_PLUGIN_PATH=../../target/release gst-inspect-1.0 $@

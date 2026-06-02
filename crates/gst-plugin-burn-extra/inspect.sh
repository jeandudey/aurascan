#!/usr/bin/env bash
cargo build --release --features scrfd-embedded,sixdrepnet360-pretrained,vulkan || exit 1
GST_PLUGIN_PATH=../../target/release gst-inspect-1.0 $@

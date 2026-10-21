#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
# SPDX-License-Identifier: MIT
#
# MJPEG webcam -> rocJPEG (VCN) -> HIPMemory -> GLMemory (HIP/GL interop) -> glimagesink.
# The frame never goes through system memory.
#
# Usage: pixi run ./launch-rocjpegdec.sh [device] [profile: debug|release]
set -e
cd "$(dirname "$0")"

DEVICE=${1:-/dev/video0}
PROFILE=${2:-debug}

cargo build $([ "$PROFILE" = release ] && echo --release) -p gst-plugin-rocm

GST_PLUGIN_PATH=$PWD/target/$PROFILE exec gst-launch-1.0 \
    v4l2src device="$DEVICE" \
    ! image/jpeg,width=1920,height=1080,framerate=60/1 \
    ! jpegparse \
    ! rocjpegdec \
    ! hipdownload \
    ! "video/x-raw(memory:GLMemory)" \
    ! fpsdisplaysink video-sink=glimagesink sync=false

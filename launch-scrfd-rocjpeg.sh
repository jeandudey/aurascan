#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
# SPDX-License-Identifier: MIT
#
# MJPEG webcam -> rocJPEG (VCN) -> HIP scale to 640x640 -> SCRFD -> ByteTrack -> face selector,
# then two branches: the frame with the tracked bbox overlay, and the face crop going through
# 6DRepNet360 (head pose). The angles are logged with GST_DEBUG=sixdrepnet360tensordec:5.
#
# Usage: pixi run ./launch-scrfd-rocjpeg.sh [device] [profile: debug|release]
set -e
cd "$(dirname "$0")"

DEVICE=${1:-/dev/video0}
PROFILE=${2:-debug}

cargo build $([ "$PROFILE" = release ] && echo --release) -p gst-plugin-rocm -p gst-plugin-aurascan

GST_DEBUG=${GST_DEBUG:-sixdrepnet360tensordec:5} GST_PLUGIN_PATH=$PWD/target/$PROFILE exec gst-launch-1.0 \
    v4l2src device="$DEVICE" \
    ! image/jpeg,width=1920,height=1080,framerate=60/1 \
    ! jpegparse \
    ! rocjpegdec \
    ! hipconvertscale \
    ! "video/x-raw(memory:HIPMemory),format=RGB,width=640,height=640" \
    ! hipdownload \
    ! video/x-raw,format=RGB \
    ! onnxinference execution-provider=cpu optimization-level=enable-all intra-op-threads=4 model-file=models/scrfd_500m.onnx \
    ! scrfdtensordec \
    ! bytetracker \
    ! faceselector \
    ! tee name=t \
    t. ! queue \
       ! videoconvert \
       ! fpsdisplaysink sync=false \
    t. ! queue leaky=downstream max-size-buffers=1 \
       ! videocropscale \
       ! video/x-raw,width=224,height=224 \
       ! onnxinference execution-provider=cpu optimization-level=enable-all intra-op-threads=8 model-file=models/sixdrepnet360.onnx \
       ! sixdrepnet360tensordec \
       ! autovideosink sync=false

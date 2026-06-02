#!/usr/bin/env bash
#cargo build --release --features scrfd-embedded,vulkan || exit 1
#GST_DEBUG=scrfdtensordec:9 \
#GST_DEBUG="GST_TRACER:7" GST_TRACERS="latency(flags=element)" \
#GST_DEBUG=burnextra-scrfdinference:9 \
GST_PLUGIN_PATH=../../target/release \
  gst-launch-1.0 \
      v4l2src \
    ! image/jpeg,width=1920,height=1080,framerate=30/1 \
    ! jpegdec \
    ! videoconvert \
    ! videoscale \
    ! video/x-raw,format=RGB,width=640,height=640 \
    ! queue \
    ! burnextra-scrfdinference backend-type=vulkan \
    ! scrfdtensordec \
    ! objectdetectionoverlay \
    ! videoconvert \
    ! fpsdisplaysink sync=false
#GST_PLUGIN_PATH=../../target/release \
#  gst-launch-1.0 \
#      v4l2src device=/dev/video2 \
#    ! video/x-raw,format=GRAY8,width=340,height=340,framerate=30/1 \
#    ! videoconvert \
#    ! videoscale \
#    ! video/x-raw,format=RGB,width=320,height=320 \
#    ! burnextra-scrfdinference backend-type=vulkan model-type=scrfd34g \
#    ! scrfdtensordec \
#    ! objectdetectionoverlay \
#    ! videoconvert \
#    ! fpsdisplaysink sync=false

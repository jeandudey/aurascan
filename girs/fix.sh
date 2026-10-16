#!/bin/bash
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
# SPDX-License-Identifier: MIT
#
# Fixups for the GIR files in this directory, same structure as
# https://gitlab.freedesktop.org/gstreamer/gir-files-rs/-/blob/main/fix.sh
set -x -e

cd "$(dirname "$0")"

# conda-forge installs xmlstarlet as `xml`.
if ! command -v xmlstarlet > /dev/null; then
    xmlstarlet() { xml "$@"; }
fi

# Strip source locations, same as gir-files-rs, to keep the diffs between
# GStreamer versions small.
for file in *.gir; do
    xmlstarlet ed -L \
        -d '//_:doc/@line' \
        -d '//_:doc/@filename' \
        -d '///_:source-position' \
        -d '//_:doc-version' \
        -d '//_:method-inline' \
        "$file"
done

# Replace the native HIP types by gpointers: hipStream_t and hipTextureObject_t
# are pointers defined in <hip/hip_runtime_api.h>, which the GIR doesn't know about.
xmlstarlet ed --inplace \
    --update '//_:type[@c:type="hipStream_t"]/@c:type' \
    --value gpointer \
    --update '//_:type[@c:type="hipTextureObject_t*"]/@c:type' \
    --value 'gpointer*' \
    GstHip-1.0.gir

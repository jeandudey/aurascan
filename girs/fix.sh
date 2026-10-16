#!/bin/bash
set -x -e

# https://github.com/gtk-rs/gir-files/blob/master/reformat.sh
# `///` used as `//` not works in Windows in this case
for file in *.gir; do
    xmlstarlet ed -L \
        -d '//_:doc/@line' \
        -d '//_:doc/@filename' \
        -d '///_:source-position' \
        -d '//_:doc-version' \
        -d '//_:method-inline' \
        "$file"
done

# replace native HIP types to gpointers
xmlstarlet ed --inplace \
    --update '//_:type[@c:type="hipStream_t"]/@c:type' \
    --value gpointer \
    --update '//_:type[@c:type="hipTextureObject_t*"]/@c:type' \
    --value 'gpointer*' \
    GstHip-1.0.gir

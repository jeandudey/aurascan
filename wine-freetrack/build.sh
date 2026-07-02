#!/usr/bin/env bash

meson setup builddir-wine64 --cross-file cross-wine64.txt
ninja -C builddir-wine64

meson setup builddir-mingw64 --cross-file cross-mingw64.txt
ninja -C builddir-mingw64

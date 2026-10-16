<!--
SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
SPDX-License-Identifier: MIT
-->

# Plan: `rocjpegdec` (gst-plugin-rocm)

Hardware JPEG decoder element: `image/jpeg` in, raw video out in
**`memory:HIPMemory`** (upstream `GstHipMemory`, `libgsthip-1.0`, GStreamer
>= 1.28), decoded on the AMD VCN engine through rocJPEG.

## Status

Implemented: `gstreamer-hip-sys`/`gstreamer-hip` (GIR generated), `rocjpeg`, and
`rocjpegdec` (see `src/rocjpegdec/imp.rs`). The sections below are the original plan; the
actual implementation differs in the following ways:

- Output formats are NV12/I420 (4:2:0), Y42B (4:2:2), Y444 (4:4:4) and RGB. GRAY8 is not
  offered because `GstHipAllocator` can't allocate it: grayscale JPEGs are output as RGB.
- Downstream HIP buffer pools are reused as is (the pool has no way to query its device).
- The element holds no lock while asking for the HIP device (it re-enters the element).

## Current state (original plan)

- `crates/rocjpeg-sys`: bindgen bindings + link test (done).
- `crates/rocjpeg`, `crates/gst-plugin-rocm` (+ `src/rocjpegdec/{mod,imp}.rs`):
  empty skeletons, `Cargo.toml` has no dependencies.
- No Rust bindings exist for `gsthip` (gstreamer-rs 0.25 has none), and the
  GIR file `GstHip-1.0.gir` is available, and
  `pkg-config` provides `gstreamer-hip-1.0` in the pixi env
  (`include/gstreamer-1.0/gst/hip`, `libgsthip-1.0.so`).

## Steps

### 1. `crates/gstreamer-hip-sys` + `crates/gstreamer-hip` (generated from GIR)
- Generate both crates the gstreamer-rs way with
  [`gir`](https://github.com/gtk-rs/gir) (built from git, the crates.io
  `gir` is unrelated) from `GstHip-1.0.gir`, which ships in the pixi env at
  `.pixi/envs/default/share/gir-1.0/` (dependencies `Gst`, `GstBase`,
  `GstVideo` GIRs come from the same directory).
- Layout: `crates/gstreamer-hip-sys/{Cargo.toml,Gir.toml,src/}` generated with
  `gir -m sys`, and `crates/gstreamer-hip/{Cargo.toml,Gir.toml,src/}`
  generated with `gir` (work mode), plus hand-written `src/*.rs` where the
  generator can't help. Pass `-d <gir dir>` for the GIR search path; no
  `gir-files` submodule.
- Both depend on the existing `gstreamer-sys`/`gstreamer-video-sys`/
  `gstreamer-base-sys` and `gstreamer`/`gstreamer-video` crates (0.25).
- Expected `Gir.toml` work: `hipError_t`, `hipStream_t`,
  `hipTextureObject_t`, `HIPfilter_mode`, `HIPaddress_mode` are plain HIP
  types, not GLib ones: add `[[object]]` entries with `ignore = true` for
  the functions using them (`gst_hip_memory_get_texture`,
  `gst_hip_device_get_attribute`, ...) or map the types to `libc`/hand-made
  ones; `GstHipMemory` embeds `GstVideoInfo` (check the sys output).
- Only expose what the element needs in the safe crate: `HipDevice`,
  `HipStream`, `HipEvent`, `HipBufferPool`, `HipAllocator`/memory helpers,
  context helpers (`gst_hip_ensure_element_data`, `handle_set_context`,
  `handle_context_query`, `context_new_hip_device`), `HipVendor`.

### 2. (merged into step 1)

### 3. `crates/rocjpeg` (safe wrapper over `rocjpeg-sys`)
- `Decoder::new(Backend, device_id)` (`rocJpegCreate`/`Destroy` on `Drop`).
- `Stream::parse(&[u8])` (`rocJpegStreamCreate/Parse/Destroy`).
- `Decoder::image_info(&Stream) -> ImageInfo { num_components, subsampling,
  widths, heights }`.
- `Decoder::decode(&Stream, OutputFormat, &mut Image)` with `Image` holding
  raw device channel pointers + pitches (`RocJpegImage`); lifetimes tie it
  to the device memory it points into (unsafe constructor from raw pointers
  since the memory is owned by `GstHipMemory`).
- `Error` from `RocJpegStatus` using `rocJpegGetErrorName`; `Result<T>`.
- Unit tests: stream create/parse of a small bundled JPEG (no GPU needed for
  parse + image info with the hybrid backend where available).

### 4. `crates/gst-plugin-rocm`
- `Cargo.toml`: copy the `gst-plugin-aurascan` pattern (`cdylib`+`rlib`, `lib
  name = "gstrocm"`, capi metadata, `build.rs` with
  `gst-plugin-version-helper`), deps: `gst`, `gst-base`, `gst-video`,
  `rocjpeg`, `gstreamer-hip`, `eyre`; add to `[workspace.dependencies]`.
- `src/lib.rs`: `plugin_define!(rocm, ...)`, `rocjpegdec::register`.
- `src/rocjpegdec/mod.rs`: `glib::wrapper!` of `RocJpegDec`, element name
  `rocjpegdec`, rank `NONE` initially (or `PRIMARY` once stable).
- `src/rocjpegdec/imp.rs`: subclass `gst_video::VideoDecoder`:
  - **Pad templates**: sink `image/jpeg, width/height ranges,
    framerate`; src `video/x-raw(memory:HIPMemory), format={RGB,NV12,GRAY8,
    Y444(planar)}, ...`.
  - **Properties**: `device-id` (u32, default 0), `backend`
    (`hardware`/`hybrid`).
  - **start/stop**: `gst_hip_ensure_element_data(vendor=AMD, device_id)`
    for the device; `rocjpeg::Decoder::new`; set context on `set_context`;
    answer `GST_QUERY_CONTEXT` via `handle_context_query`.
  - **set_format**: store sink caps; defer output state until the first
    frame (size/subsampling come from the stream header).
  - **handle_frame**: map input buffer read, `Stream::parse`, `image_info`,
    pick `OutputFormat` from the negotiated src format (NV12 only for 4:2:0
    -> `NATIVE`; RGB -> `RGB`; GRAY8 -> `Y`), negotiate/`set_output_state`
    if size or format changed, `decide_allocation` / `propose_allocation`
    with `HipBufferPool` (video info, `GST_CAPS_FEATURE_MEMORY_HIP_MEMORY`),
    acquire output buffer, map the `GstHipMemory` with `GST_MAP_WRITE_HIP`
    to get the device pointer(s) + strides, fill `RocJpegImage`, call
    `decode`, then `gst_hip_memory_sync` (or record the memory event) so
    downstream sees finished data, `finish_frame`.
  - Flush/drain: nothing buffered (one JPEG = one frame).
  - Errors: map `Error` to `gst::element_error!` (`BAD_JPEG` ->
    `Decode`, `HW_JPEG_DECODER_NOT_SUPPORTED` -> `Failed` with a hint to
    try `backend=hybrid`).

### 5. Packaging / CI
- Root `Cargo.toml`: add the new crates to `members` (and tidy the
  one-line `members` formatting that was left on `matio-sys`/`gst-plugin-aurascan`).
- `pixi.toml`: `gstreamer-hip` is part of `gst-plugins-bad` (already a
  dependency); make sure `LD_LIBRARY_PATH`/`GST_PLUGIN_PATH` expose
  `libgstrocm.so`. `aurascan` recipe: add `rocjpeg` to `host`.

## Verification
1. `pixi run cargo build -p gstreamer-hip-sys -p gstreamer-hip -p rocjpeg
   -p gst-plugin-rocm`
2. `pixi run gst-inspect-1.0 --gst-plugin-path target/debug rocjpegdec`
   shows pad templates and properties.
3. Needs an AMD GPU with VCN JPEG support:
   `gst-launch-1.0 filesrc location=mug.jpg ! jpegparse ! rocjpegdec !
   hipdownload ! videoconvert ! pngenc ! filesink location=out.png`
   (use `hipdownload` from `gst-plugins-bad` to copy to system memory;
   compare with `jpegdec` output).
4. Test with a JPEG stream (`multifilesrc ! jpegparse ! rocjpegdec`) and with
   4:2:0, 4:2:2, 4:4:4 and grayscale inputs (the rocJPEG sample images under
   `$CONDA_PREFIX/share/rocjpeg/images`).

## Open questions
- Which raw formats rocJPEG can write for non-4:2:0 sources on the target GPU
  (RGB conversion is VCN-side only on MI300+, HIP kernels elsewhere).
- ~~Whether `rocJpegDecode` is synchronous~~: confirmed from the 7.2.4 source
  (`src/rocjpeg_decoder.cpp`): `Decode` calls `vaSyncSurface`, runs the HIP
  conversion/copy kernels on its own private stream (`hipStreamCreate` in
  `InitHIP`) and finishes with `hipStreamSynchronize` on it, so the output is
  complete when it returns and no `GstHipEvent` is needed. `hipSetDevice` is
  only called in `rocJpegCreate`, and HIP's current device is per-thread, so
  call `GstHipDevice::set_current` (or create the decoder) on the streaming
  thread that decodes.

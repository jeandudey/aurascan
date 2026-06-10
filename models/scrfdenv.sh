#!/usr/bin/env bash

uv venv .venvtools --allow-existing
source .venvtools/bin/activate
uv pip install gdown onnx
deactivate

uv venv --python 3.8 --allow-existing .venvscrfd
source .venvscrfd/bin/activate

uv pip install \
  torch==1.8.1 \
  torchvision==0.9.1 \
  mmcv-full==1.3.3 \
  onnx==1.9.0 \
  onnxruntime==1.8.0 \
  onnx-simplifier==0.3.6 \
  numpy==1.23.5 \
  Pillow \
  terminaltables \
  pycocotools \
  setuptools \
  scipy==1.10.1 \
  tqdm \
  --find-links https://download.openmmlab.com/mmcv/dist/cpu/torch1.8.0/index.html
uv pip install -r insightface/detection/scrfd/requirements/build.txt
uv pip install -v -e insightface/detection/scrfd --no-deps --no-build-isolation

patchelf .venvscrfd/lib/python3.8/site-packages/onnxruntime/capi/onnxruntime_pybind11_state.cpython-38-x86_64-linux-gnu.so \
  --clear-execstack

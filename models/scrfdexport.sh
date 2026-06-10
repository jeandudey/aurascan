#!/usr/bin/env bash

source .venvscrfd/bin/activate

scrfd2onnx() {
    PROTOCOL_BUFFERS_PYTHON_IMPLEMENTATION=python \
    python3 insightface/detection/scrfd/tools/scrfd2onnx.py \
        "insightface/detection/scrfd/configs/scrfd/$1.py" \
        "pth/$2.pth" \
        --input-img insightface/python-package/insightface/data/images/t1.jpg
}

scrfd2onnx scrfd_1g SCRFD_1G
scrfd2onnx scrfd_2.5g SCRFD_2_5G
scrfd2onnx scrfd_10g SCRFD_10G
scrfd2onnx scrfd_34g SCRFD_34G
scrfd2onnx scrfd_500m_bnkps SCRFD_500M_KPS
scrfd2onnx scrfd_2.5g_bnkps SCRFD_2_5G_KPS
scrfd2onnx scrfd_10g_bnkps SCRFD_10G_KPS

mv insightface/detection/scrfd/onnx/*.onnx onnx/

deactivate

source .venvtools/bin/activate

for f in onnx/scrfd_*.onnx; do
    python3 scrfdtransform.py "$f"
done

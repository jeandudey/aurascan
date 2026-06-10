# SPDX-FileCopyrightText: 2024 Thorsten Hempel <tho.hemp@protonmail.com>
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
#
# SPDX-License-Identifier: MIT

import torch

from sixdrepnet360.model import SixDRepNet360


def export(
    weights: str = "6DRepNet360_Full-Rotation_300W_LP+Panoptic.pth",
    out: str = "sixdrepnet360.onnx",
):
    model = SixDRepNet360()
    _ = model.load_state_dict(torch.load(weights))  # pyright: ignore[reportAny]
    _ = model.eval()

    dummy = torch.randn(1, 3, 224, 224)

    _ = torch.onnx.export(  # pyright: ignore[reportUnknownMemberType]
        model,
        (dummy,),
        out,
        input_names=["input"],
        output_names=["output"],
        dynamic_axes={"input": {0: "batch"}, "output": {0: "batch_size"}},
        dynamo=True,
    )
    print(f"exported -> {out}")


if __name__ == "__main__":
    export()

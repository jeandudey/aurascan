# SPDX-FileCopyrightText: 2024 Thorsten Hempel <tho.hemp@protonmail.com>
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
#
# SPDX-License-Identifier: MIT

from typing import cast

import torch
import torch.nn as nn
from torchvision.models.resnet import (  # pyright: ignore[reportMissingTypeStubs]
    Bottleneck,
)
from typing_extensions import override

from sixdrepnet360.ops import compute_rotation_matrix_from_ortho6d


class SixDRepNet360(nn.Module):
    inplanes: int
    conv1: nn.Conv2d
    bn1: nn.BatchNorm2d
    relu: nn.ReLU
    maxpool: nn.MaxPool2d
    layer1: nn.Sequential
    layer2: nn.Sequential
    layer3: nn.Sequential
    layer4: nn.Sequential
    avgpool: nn.AvgPool2d
    linear_reg: nn.Linear

    def __init__(
        self,
        fc_layers: int = 6,
    ):
        self.inplanes = 64
        super(SixDRepNet360, self).__init__()
        self.conv1 = nn.Conv2d(3, 64, kernel_size=7, stride=2, padding=3, bias=False)
        self.bn1 = nn.BatchNorm2d(64)
        self.relu = nn.ReLU(inplace=True)
        self.maxpool = nn.MaxPool2d(kernel_size=3, stride=2, padding=1)
        self.layer1 = self._make_layer(64, 3)
        self.layer2 = self._make_layer(128, 4, stride=2)
        self.layer3 = self._make_layer(256, 6, stride=2)
        self.layer4 = self._make_layer(512, 3, stride=2)
        self.avgpool = nn.AvgPool2d(7)

        self.linear_reg = nn.Linear(512 * Bottleneck.expansion, 6)

    def _make_layer(self, planes: int, blocks: int, stride: int = 1) -> nn.Sequential:
        downsample = None
        if stride != 1 or self.inplanes != planes * Bottleneck.expansion:
            downsample = nn.Sequential(
                nn.Conv2d(
                    self.inplanes,
                    planes * Bottleneck.expansion,
                    kernel_size=1,
                    stride=stride,
                    bias=False,
                ),
                nn.BatchNorm2d(planes * Bottleneck.expansion),
            )

        layers: list[Bottleneck] = []
        layers.append(Bottleneck(self.inplanes, planes, stride, downsample))
        self.inplanes = planes * Bottleneck.expansion
        for _ in range(1, blocks):
            layers.append(Bottleneck(self.inplanes, planes))

        return nn.Sequential(*layers)

    @override
    def forward(self, x: torch.Tensor) -> torch.Tensor:
        x = cast(torch.Tensor, self.conv1(x))
        x = cast(torch.Tensor, self.bn1(x))
        x = cast(torch.Tensor, self.relu(x))
        x = cast(torch.Tensor, self.maxpool(x))

        x = cast(torch.Tensor, self.layer1(x))
        x = cast(torch.Tensor, self.layer2(x))
        x = cast(torch.Tensor, self.layer3(x))
        x = cast(torch.Tensor, self.layer4(x))

        x = cast(torch.Tensor, self.avgpool(x))
        x = x.view(x.size(0), -1)

        x = cast(torch.Tensor, self.linear_reg(x))
        return compute_rotation_matrix_from_ortho6d(x)

# SPDX-FileCopyrightText: 2024 Thorsten Hempel <tho.hemp@protonmail.com>
# SPDX-FileCopyrightText: 2026 Jean-Pierre De Jesus DIAZ <me@jeandudey.tech>
#
# SPDX-License-Identifier: MIT

import torch


# batch*n
def normalize_vector(v: torch.Tensor) -> torch.Tensor:
    batch = v.shape[0]
    v_mag = torch.sqrt(v.pow(2).sum(1))  # batch
    gpu = v_mag.get_device()
    if gpu < 0:
        eps = torch.autograd.Variable(torch.FloatTensor([1e-8])).to(torch.device("cpu"))
    else:
        eps = torch.autograd.Variable(torch.FloatTensor([1e-8])).to(
            torch.device("cuda:%d" % gpu)
        )
    v_mag = torch.max(v_mag, eps)
    v_mag = v_mag.view(batch, 1).expand(batch, v.shape[1])
    v = v / v_mag
    return v


# u, v batch*n
def cross_product(u: torch.Tensor, v: torch.Tensor) -> torch.Tensor:
    batch = u.shape[0]
    # print (u.shape)
    # print (v.shape)
    i = u[:, 1] * v[:, 2] - u[:, 2] * v[:, 1]
    j = u[:, 2] * v[:, 0] - u[:, 0] * v[:, 2]
    k = u[:, 0] * v[:, 1] - u[:, 1] * v[:, 0]

    return torch.cat(
        (i.view(batch, 1), j.view(batch, 1), k.view(batch, 1)), 1
    )  # batch*3


# poses batch*6
# poses
def compute_rotation_matrix_from_ortho6d(poses: torch.Tensor) -> torch.Tensor:
    x_raw = poses[:, 0:3]  # batch*3
    y_raw = poses[:, 3:6]  # batch*3

    x = normalize_vector(x_raw)  # batch*3
    z = cross_product(x, y_raw)  # batch*3
    z = normalize_vector(z)  # batch*3
    y = cross_product(z, x)  # batch*3

    x = x.view(-1, 3, 1)
    y = y.view(-1, 3, 1)
    z = z.view(-1, 3, 1)
    matrix = torch.cat((x, y, z), 2)  # batch*3*3
    return matrix

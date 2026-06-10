#!/usr/bin/env bash

source .venvtools/bin/activate
[ -f pth/SCRFD_500M.pth ] || gdown 'https://drive.google.com/file/d/1OX0i_vWDp1Fp-ZynOUMZo-q1vB5g1pTN/view?usp=sharing' -O pth/SCRFD_500M.pth
[ -f pth/SCRFD_1G.pth ] || gdown 'https://drive.google.com/file/d/1OX0i_vWDp1Fp-ZynOUMZo-q1vB5g1pTN/view?usp=sharing' -O pth/SCRFD_1G.pth
[ -f pth/SCRFD_2_5G.pth ] || gdown 'https://drive.google.com/file/d/1wgg8GY2vyP3uUTaAKT0_MSpAPIhmDsCQ/view?usp=sharing' -O pth/SCRFD_2_5G.pth
[ -f pth/SCRFD_10G.pth ] || gdown 'https://drive.google.com/file/d/1kUYa0s1XxLW37ZFRGeIfKNr9L_4ScpOg/view?usp=sharing' -O pth/SCRFD_10G.pth
[ -f pth/SCRFD_34G.pth ] || gdown 'https://drive.google.com/file/d/1w9QOPilC9EhU0JgiVJoX0PLvfNSlm1XE/view?usp=sharing' -O pth/SCRFD_34G.pth
[ -f pth/SCRFD_500M_KPS.pth ] || gdown 'https://drive.google.com/file/d/1TXvKmfLTTxtk7tMd2fEf-iWtAljlWDud/view?usp=sharing' -O pth/SCRFD_500M_KPS.pth
[ -f pth/SCRFD_2_5G_KPS.pth ] || gdown 'https://drive.google.com/file/d/1KtOB9TocdPG9sk_S_-1QVG21y7OoLIIf/view?usp=sharing' -O pth/SCRFD_2_5G_KPS.pth
[ -f pth/SCRFD_10G_KPS.pth ] || gdown 'https://drive.google.com/file/d/1-2uy0tgkenw6ZLxfKV1qVhmkb5Ep_5yx/view?usp=sharing' -O pth/SCRFD_10G_KPS.pth

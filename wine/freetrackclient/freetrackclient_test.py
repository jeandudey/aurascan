#!/usr/bin/env python3

import os
import subprocess
import sys

dll = sys.argv[1]
test_exe = sys.argv[2]
key = r"HKEY_CURRENT_USER\Software\Freetrack\FreetrackClient"

if sys.platform == "win32":
    path = os.path.dirname(os.path.abspath(dll))
    subprocess.run(
        ["reg", "add", key, "/v", "Path", "/t", "REG_SZ", "/d", path, "/f"], check=True
    )
    subprocess.run([test_exe], check=True)
else:
    path = subprocess.check_output(
        ["winepath", "--windows", os.path.abspath(os.path.dirname(dll))], text=True
    ).strip()
    subprocess.run(
        ["wine64", "reg", "add", key, "/v", "Path", "/t", "REG_SZ", "/d", path, "/f"],
        check=True,
    )
    subprocess.run(["wine64", test_exe], check=True)

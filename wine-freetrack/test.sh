#!/usr/bin/env bash

cp builddir-wine64/freetrackclient/freetrackclient.so builddir-mingw64/freetrackclient

WINEDLLPATH=$PWD/builddir-mingw64/freetrackclient \
WINEDEBUG=+freetrack \
wine64 builddir-mingw64/freetrackclient/loadtest.exe

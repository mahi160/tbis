#!/bin/sh
set -eu

apt-get update
apt-get install -y --no-install-recommends \
    build-essential cmake pkg-config \
    libfontconfig-dev libmpv-dev libxcb1-dev libxkbcommon-dev libxkbcommon-x11-dev

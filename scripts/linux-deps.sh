#!/usr/bin/env bash
# linux-deps.sh: the packages SDL3 needs to build from source on Debian or Ubuntu, with both
# Wayland (preferred) and X11, and ALSA, PulseAudio and PipeWire for sound.
set -euo pipefail
sudo apt-get update -qq
sudo apt-get install -y -qq --no-install-recommends \
  cmake ninja-build pkg-config \
  libwayland-dev libxkbcommon-dev wayland-protocols libdecor-0-dev libegl1-mesa-dev \
  libx11-dev libxext-dev libxrandr-dev libxcursor-dev libxfixes-dev libxi-dev libxss-dev libxtst-dev \
  libasound2-dev libpulse-dev libpipewire-0.3-dev \
  libdrm-dev libgbm-dev libudev-dev libdbus-1-dev libibus-1.0-dev

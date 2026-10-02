#!/usr/bin/env sh
set -eux

apt-get update

apt-get install \
  --assume-yes \
  --no-install-recommends \
  build-essential \
  ca-certificates \
  clang \
  curl \
  g++-aarch64-linux-gnu \
  g++-arm-linux-gnueabihf \
  git \
  jq \
  libssl-dev \
  pkg-config \
  python3-venv \
  ssh \
  sshpass

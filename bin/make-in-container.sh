#!/usr/bin/env sh
set -eux

REPO_ROOT="$(CDPATH='' cd -- "$(dirname -- "$0")/.." && pwd)"

TAG='tailnet-acap-dev'

WORKSPACE="/workspaces/$(basename -- "${REPO_ROOT}")"

docker build \
  --tag "${TAG}" \
  --file "${REPO_ROOT}/.devcontainer/Dockerfile" \
  "${REPO_ROOT}"

docker run --rm --volume tailnet-acap-target:/target "${TAG}" chmod 0777 /target

mkdir -p "${REPO_ROOT}/target-container/acap"

docker run \
  --rm \
  --user "$(id -u):$(id -g)" \
  --volume "${REPO_ROOT}:${WORKSPACE}" \
  --volume tailnet-acap-cargo-registry:/usr/local/cargo/registry \
  --volume tailnet-acap-cargo-git:/usr/local/cargo/git \
  --volume tailnet-acap-target:/target \
  --volume "${REPO_ROOT}/target-container:${WORKSPACE}/target" \
  --workdir "${WORKSPACE}" \
  --env AXIS_DEVICE_ARCH \
  --env AXIS_DEVICE_HTTP_PORT \
  --env AXIS_DEVICE_HTTPS_PORT \
  --env AXIS_DEVICE_IP \
  --env AXIS_DEVICE_PASS \
  --env AXIS_DEVICE_SSH_PORT \
  --env AXIS_DEVICE_SSH_USER \
  --env AXIS_DEVICE_USER \
  --env AXIS_PACKAGE \
  "${TAG}" \
  make "$@"

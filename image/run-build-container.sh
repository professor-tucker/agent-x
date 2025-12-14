#!/usr/bin/env bash
set -euo pipefail

# Build and run a privileged container to create the ph-agent ISO.
# Usage: ./run-build-container.sh

IMAGE_NAME=ph-agent-builder:latest
WORKDIR_HOST=$(pwd)/..

echo "Building container image ${IMAGE_NAME}..."
docker build -t ${IMAGE_NAME} -f Dockerfile ..

echo "Running build inside privileged container (this requires Docker installed on host)..."
docker run --rm --privileged -v "${WORKDIR_HOST}:/work" -w /work/agent-x/image ${IMAGE_NAME} bash -lc "/work/agent-x/image/build-image.sh"

echo "If successful, ISO will be in $(pwd)"

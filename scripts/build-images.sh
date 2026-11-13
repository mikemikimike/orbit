#!/usr/bin/env bash
# Build ORBIT images for the cluster architecture (linux/amd64).
# Usage: REGISTRY=192.168.1.201:5000 TAG=0.1.0 ./scripts/build-images.sh
set -euo pipefail

REGISTRY="${REGISTRY:-192.168.1.201:5000}"
TAG="${TAG:-0.1.0}"
PLATFORM="${PLATFORM:-linux/amd64}"

cd "$(dirname "$0")/.."

echo "==> Building ORBIT images for ${PLATFORM}, registry ${REGISTRY}, tag ${TAG}"

for component in api worker web launcher; do
  dockerfile="deploy/docker/${component}.Dockerfile"
  image="${REGISTRY}/orbit-${component}:${TAG}"
  echo "--- ${component} -> ${image}"
  docker build --platform "${PLATFORM}" -f "${dockerfile}" -t "${image}" .
  docker tag "${image}" "${REGISTRY}/orbit-${component}:latest"
done

# Harness-specific worker images (depend on the base worker image).
for component in worker-opencode worker-pi; do
  dockerfile="deploy/docker/${component}.Dockerfile"
  image="${REGISTRY}/orbit-${component}:${TAG}"
  echo "--- ${component} -> ${image}"
  docker build --platform "${PLATFORM}" -f "${dockerfile}"     --build-arg "WORKER_BASE=${REGISTRY}/orbit-worker:${TAG}"     -t "${image}" .
  docker tag "${image}" "${REGISTRY}/orbit-${component}:latest"
done

echo "==> Done. Push with: REGISTRY=${REGISTRY} TAG=${TAG} ./scripts/push-images.sh"

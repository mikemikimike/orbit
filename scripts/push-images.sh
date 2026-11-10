#!/usr/bin/env bash
# Push ORBIT images to the private registry.
# Usage: REGISTRY=192.168.1.201:5000 TAG=0.1.0 ./scripts/push-images.sh
set -euo pipefail

REGISTRY="${REGISTRY:-192.168.1.201:5000}"
TAG="${TAG:-0.1.0}"
# Some environments have the registry trusted under a DNS name while the cluster pulls
# by IP. PUSH_REGISTRY lets you push through the trusted name; the repository path is
# identical.
PUSH_REGISTRY="${PUSH_REGISTRY:-${REGISTRY}}"

cd "$(dirname "$0")/.."

echo "==> Pushing ORBIT images via ${PUSH_REGISTRY} (cluster pulls ${REGISTRY})"
for component in api worker web worker-opencode worker-pi; do
  source_image="${REGISTRY}/orbit-${component}:${TAG}"
  push_image="${PUSH_REGISTRY}/orbit-${component}:${TAG}"
  push_latest="${PUSH_REGISTRY}/orbit-${component}:latest"
  latest_source="${REGISTRY}/orbit-${component}:latest"

  if [ "${source_image}" != "${push_image}" ]; then
    docker tag "${source_image}" "${push_image}"
  fi
  docker tag "${source_image}" "${latest_source}"
  if [ "${latest_source}" != "${push_latest}" ]; then
    docker tag "${source_image}" "${push_latest}"
  fi

  docker push "${push_image}"
  docker push "${push_latest}"
done
echo "==> Images pushed."

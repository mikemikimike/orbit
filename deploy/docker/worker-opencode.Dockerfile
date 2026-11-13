# syntax=docker/dockerfile:1.7
# OpenCode-enabled execution worker.
#
# The base worker image is extended with the OpenCode CLI (not bundled into the control
# plane). In Kubernetes this runs as a separate Deployment with
# ORBIT_WORKER_HARNESSES=opencode so only this image claims OpenCode executions.
ARG WORKER_BASE=orbit-worker:latest
FROM ${WORKER_BASE}

USER root
RUN apt-get update \
    && apt-get upgrade -y \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

# Tested version: OpenCode v2.2.5 (see crates/orbit-harness/tests/opencode_e2e.rs).
ARG OPENCODE_VERSION=2.3.0
ARG OPENCODE_ARCH=x64
RUN mkdir -p /usr/local/lib/opencode \
      /home/orbit/.local/share/opencode \
      /home/orbit/.config/opencode \
      /home/orbit/.cache \
    && curl -fsSL --retry 3 \
       "https://github.com/anomalyco/opencode/releases/download/v${OPENCODE_VERSION}/opencode-linux-${OPENCODE_ARCH}.tar.gz" \
       -o /tmp/opencode.tar.gz \
    && tar -xzf /tmp/opencode.tar.gz -C /usr/local/bin opencode \
    && chmod 0755 /usr/local/bin/opencode \
    && rm -f /tmp/opencode.tar.gz \
    && chown -R 10001:10001 /home/orbit \
    && opencode --version

ENV HOME=/home/orbit \
    XDG_DATA_HOME=/home/orbit/.local/share \
    XDG_CONFIG_HOME=/home/orbit/.config \
    XDG_CACHE_HOME=/home/orbit/.cache \
    ORBIT_WORKER_HARNESSES=opencode

USER 10001:10001
ENTRYPOINT ["/usr/local/bin/orbit-worker"]

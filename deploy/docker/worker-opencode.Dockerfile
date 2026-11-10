# syntax=docker/dockerfile:1.7
# OpenCode-enabled execution worker.
#
# The base worker image is extended with the OpenCode CLI (not bundled into the control
# plane). In Kubernetes this runs as a separate Deployment with
# ORBIT_WORKER_RUNTIMES=opencode so only this image claims OpenCode executions.
ARG WORKER_BASE=192.168.1.201:5000/orbit-worker:0.1.3
FROM ${WORKER_BASE}

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

# Tested version: OpenCode v2.2.5 (see crates/orbit-runtime/tests/opencode_e2e.rs).
ARG OPENCODE_VERSION=2.2.5
RUN mkdir -p /usr/local/lib/opencode \
      /home/orbit/.local/share/opencode \
      /home/orbit/.config/opencode \
    && curl -fsSL --retry 3 \
       "https://github.com/anomalyco/opencode/releases/download/v${OPENCODE_VERSION}/opencode-linux-x64.tar.gz" \
       -o /tmp/opencode.tar.gz \
    && tar -xzf /tmp/opencode.tar.gz -C /usr/local/bin opencode \
    && chmod 0755 /usr/local/bin/opencode \
    && rm -f /tmp/opencode.tar.gz \
    && chown -R 10001:10001 /home/orbit \
    && opencode --version

ENV HOME=/home/orbit \
    XDG_DATA_HOME=/home/orbit/.local/share \
    XDG_CONFIG_HOME=/home/orbit/.config \
    ORBIT_WORKER_RUNTIMES=opencode

USER 10001:10001
ENTRYPOINT ["/usr/local/bin/orbit-worker"]

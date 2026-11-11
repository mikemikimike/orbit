# syntax=docker/dockerfile:1.7
# Pi-enabled execution worker. Pi is a Node package; the Rust worker binaries come from
# the base worker image. Auth is mounted externally (PI_CODING_AGENT_DIR).
ARG WORKER_BASE=192.168.1.201:5000/orbit-worker:0.5.1
FROM node:22-bookworm-slim AS node
FROM ${WORKER_BASE}

USER root
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl git \
    && rm -rf /var/lib/apt/lists/*

COPY --from=node /usr/local/bin/node /usr/local/bin/node
COPY --from=node /usr/local/lib/node_modules /usr/local/lib/node_modules
RUN ln -sf /usr/local/lib/node_modules/npm/bin/npm-cli.js /usr/local/bin/npm \
    && ln -sf /usr/local/lib/node_modules/npm/bin/npx-cli.js /usr/local/bin/npx \
    && npm install -g @earendil-works/pi-coding-agent@1.5.0 \
    && npm cache clean --force \
    && pi --version

RUN mkdir -p /home/orbit/.pi/agent \
    && chown -R 10001:10001 /home/orbit

ENV HOME=/home/orbit \
    PI_CODING_AGENT_DIR=/home/orbit/.pi/agent \
    ORBIT_WORKER_HARNESSES=pi

USER 10001:10001
ENTRYPOINT ["/usr/local/bin/orbit-worker"]

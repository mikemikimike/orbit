# syntax=docker/dockerfile:1.7
# ORBIT execution-plane worker. Runtime binaries are NOT bundled here: runtime
# containers/Jobs are separate images by design (see docs/architecture/runtime-contract.md).
FROM rust:1-bookworm AS builder
WORKDIR /app

COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/orbit-domain/Cargo.toml crates/orbit-domain/Cargo.toml
COPY crates/orbit-contracts/Cargo.toml crates/orbit-contracts/Cargo.toml
COPY crates/orbit-store/Cargo.toml crates/orbit-store/Cargo.toml
COPY crates/orbit-events/Cargo.toml crates/orbit-events/Cargo.toml
COPY crates/orbit-security/Cargo.toml crates/orbit-security/Cargo.toml
COPY crates/orbit-runtime/Cargo.toml crates/orbit-runtime/Cargo.toml
COPY crates/orbit-api/Cargo.toml crates/orbit-api/Cargo.toml
COPY services/api/Cargo.toml services/api/Cargo.toml
COPY services/worker/Cargo.toml services/worker/Cargo.toml
RUN mkdir -p crates/orbit-domain/src crates/orbit-contracts/src crates/orbit-store/src \
      crates/orbit-events/src crates/orbit-security/src crates/orbit-runtime/src \
      crates/orbit-api/src services/api/src services/worker/src \
    && for d in crates/orbit-domain crates/orbit-contracts crates/orbit-store crates/orbit-events \
              crates/orbit-security crates/orbit-runtime crates/orbit-api; do echo "" > "$d/src/lib.rs"; done \
    && echo "fn main() {}" > services/api/src/main.rs \
    && echo "fn main() {}" > services/worker/src/main.rs \
    && cargo fetch --locked

COPY crates ./crates
COPY services ./services
RUN cargo build --release --locked -p orbit-worker --bin orbit-worker

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates procps \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 orbit \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin orbit \
    && mkdir -p /tmp/orbit-workspaces /workspace \
    && chown -R 10001:10001 /tmp/orbit-workspaces /workspace

COPY --from=builder /app/target/release/orbit-worker /usr/local/bin/orbit-worker
USER 10001:10001
ENV ORBIT_WORKSPACE_ROOT=/tmp/orbit-workspaces
HEALTHCHECK --interval=30s --timeout=5s --start-period=15s --retries=3 \
  CMD pgrep -f orbit-worker >/dev/null || exit 1
ENTRYPOINT ["/usr/local/bin/orbit-worker"]

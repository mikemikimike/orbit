# Deployment (Docker Compose & Kubernetes)

## Images

| Image | Dockerfile | Notes |
|---|---|---|
| `ghcr.io/orbit-platform/orbit-api` | `deploy/docker/api.Dockerfile` | multi-stage, non-root, embedded migrations |
| `ghcr.io/orbit-platform/orbit-worker` | `deploy/docker/worker.Dockerfile` | non-root, harness adapters |
| `ghcr.io/orbit-platform/orbit-web` | `deploy/docker/web.Dockerfile` | Next.js standalone output, non-root |

```bash
docker build -f deploy/docker/api.Dockerfile -t orbit-api:dev .
docker build -f deploy/docker/worker.Dockerfile -t orbit-worker:dev .
docker build -f deploy/docker/web.Dockerfile -t orbit-web:dev .
```

## Compose

`deploy/compose/compose.yaml` — full stack (db, nats, api, worker, web).
`deploy/compose/compose.dev.yaml` — infrastructure only (db, nats).

## Helm

The default values assume a private registry and an nginx ingress:

- `imageRegistry` points at your own registry (set it to your registry host)
- images are built for `linux/amd64`
- default namespace `orbit`, ingress class `nginx`, ingress host `orbit.example.com`
  (override with `--set ingress.host=…`)

```bash
# 1. Build (linux/amd64) and push images to your registry
TAG=0.7.0 ./scripts/build-images.sh
PUSH_REGISTRY=registry.example.com:5000 TAG=0.7.0 ./scripts/push-images.sh

# 2. Deploy (workers need scoped tokens)
#    Start the API first, then issue tokens:
MOCK_TOKEN=$(curl -s -X POST -H "Authorization: Bearer $DEV_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"cluster-mock","harnesses":["mock"]}' \
  http://orbit.example.com/api/v1/worker-tokens | jq -r .token)
OPENCODE_TOKEN=$(curl -s -X POST -H "Authorization: Bearer $DEV_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"cluster-opencode","harnesses":["opencode"]}' \
  http://orbit.example.com/api/v1/worker-tokens | jq -r .token)

KUBE_CONTEXT=your-cluster NAMESPACE=orbit TAG=0.7.0 \
  ./scripts/deploy.sh \
  --set worker.token="$MOCK_TOKEN" \
  --set opencodeWorker.token="$OPENCODE_TOKEN"

# Equivalent explicit Helm command
helm --kube-context your-cluster upgrade --install orbit deploy/helm/orbit \
  --namespace orbit --create-namespace \
  -f deploy/helm/orbit/values.yaml \
  --set imageRegistry=registry.example.com:5000 \
  --set api.tag=0.7.0 --set worker.tag=0.7.0 --set web.tag=0.7.0 \
  --set worker.token="$MOCK_TOKEN" --set opencodeWorker.token="$OPENCODE_TOKEN"
```

### Worker launcher (optional)

Set `launcher.enabled=true` and `launcher.token=<random>` to enable Launch/Stop/Logs.
The chart creates a namespaced Pod Role/RoleBinding for the launcher only; `orbit-api`
never gets workload RBAC. Without the launcher, Workers still supports profiles,
tokens and bootstrap manifests.

```bash
helm upgrade orbit deploy/helm/orbit -n orbit -f values.yaml \
  --set launcher.enabled=true --set launcher.token="$LAUNCHER_TOKEN"
```

Launch images must be pullable by the cluster. Each launch sets `ORBIT_API_URL` and the
profile's worker token: long-lived profiles run as a Pod, single-execution launches run
as a Job with a TTL for cleanup. Harness auth is mounted from a Secret when configured.

Ingress routes: `/docs`, `/healthz`, `/readyz` → `orbit-api`; `/api` → `orbit-web`
(the web container proxies `/api/*` to the API at harness, injecting the dev token
server-side when dev auth is enabled and forwarding OIDC/API bearer headers otherwise);
`/` → `orbit-web`.
The chart sets SSE-friendly nginx annotations (`proxy-buffering: off`, long
read/send timeouts). Point `orbit.example.com` at the ingress address (or add an
`/etc/hosts`/split-DNS entry).

The chart has been smoke-tested end-to-end on a single-node Kubernetes cluster
(ingress → web proxy → API → PostgreSQL → worker → mock harness → SSE).

### macOS "no route to host" while the cluster is up

If `curl https://KUBE_API_IP:16443/version` works but `kubectl` fails with
`dial tcp KUBE_API_IP:16443: connect: no route to host`, macOS Local Network privacy is
likely blocking the `kubectl` binary (unsigned/third-party binaries can be denied even
when the terminal app is allowed). Fix it in **System Settings → Privacy & Security →
Local Network** by enabling your terminal and `kubectl`, or run the deployment from an
allowed terminal. As a temporary workaround, forward the API through a loopback relay
started by an allowed interpreter (`127.0.0.1:16444` → `KUBE_API_IP:16443`) and use a
kubeconfig whose server is `https://127.0.0.1:16444`; kubectl then only needs loopback.

Key values:
- `externalDatabase.enabled` / `postgresql.enabled` — the chart can bundle a
  single-replica PostgreSQL for test clusters; production should use a managed database.
- `externalNats.enabled` / `nats.enabled`.
- `auth.mode` / `auth.oidc.*` — `dev` or `oidc`; issuer, client id, scopes and JWKS URL.
  OIDC can also be configured at runtime under Settings.
- `secrets.existingSecret` — chart does not manage secret material in production;
  `ORBIT_MASTER_KEY`, `ORBIT_DATABASE_URL`, OIDC client secret live there.
- `worker.*` / `opencodeWorker.*` — replicas, resources, harness allowlist and the
  scoped worker token; `opencodeWorker.enabled` adds the OpenCode worker.
- `launcher.*` — optional worker launcher (creates worker Pods/Jobs in-cluster).
- `caCertificates.*` — mount a private CA bundle into every workload and set
  `SSL_CERT_FILE` / `REQUESTS_CA_BUNDLE` / `NODE_EXTRA_CA_CERTS`.
- `ingress.*` — nginx annotations keep SSE buffering disabled.

The chart requires no cluster-admin permissions and creates a namespaced ServiceAccount.

## OpenCode harness worker

OpenCode runs in a separate execution-plane image (`deploy/docker/worker-opencode.Dockerfile`)
so the control plane and base worker do not bundle harness frameworks. The worker claims
only `harness_kind=opencode` executions (`ORBIT_WORKER_HARNESSES=opencode`).

```bash
# 1. export credentials from a logged-in OpenCode install (contains secrets)
opencode auth export > /tmp/auth-export.json
kubectl -n orbit create secret generic orbit-opencode-auth \
  --from-file=auth-export.json=/tmp/auth-export.json

# 2. enable the worker
helm upgrade orbit deploy/helm/orbit -n orbit --reuse-values \
  --set opencodeWorker.enabled=true \
  --set opencodeWorker.auth.existingSecret=orbit-opencode-auth
```

An init container imports the credentials into a writable emptyDir; the main container
runs as non-root and executes `opencode run --standalone --format json`. Tested with
OpenCode v2.3.0. Rotate credentials by recreating the Secret and restarting the
Deployment. For local Docker/OrbStack, mount the exported auth file similarly or run the
worker with `ORBIT_WORKER_HARNESSES=opencode` and a writable `HOME`.

## Bundled vs external dependencies

`values.yaml` bundles PostgreSQL and NATS for test clusters (single replica, PVC-backed).
Production uses `values-prod.example.yaml`: external PostgreSQL/NATS, OIDC, TLS and an
externally managed Secret with `database-url` and `master-key`. The bundled PostgreSQL
runs with the capabilities its entrypoint needs to chown its data directory; it is not a
production-grade database deployment.

The chart requires no cluster-admin permissions: it creates a namespaced ServiceAccount
with token automounting disabled and namespaced Deployments/Services/Ingress only.
Vault integration is env wiring only (`VAULT_ADDR`); the Vault secret backend lands in a
later phase.

## Kubernetes job-per-execution

The optional launcher (`launcher.enabled=true`) creates workloads in the target
namespace:

- **Long-lived worker profiles** are started as a `Pod` (one worker per launch).
- **Single-execution launches** are started as a `Job` with `restartPolicy: Never`,
  `backoffLimit: 0` and `ttlSecondsAfterFinished: 300`, so the pod is isolated per
  execution and cleaned up automatically.

The launcher only needs `get/list/create/delete` on `pods` and `get` on `secrets` in its
namespace (see [kubernetes-launcher.md](kubernetes-launcher.md)). The control plane
never receives cluster credentials.

The base worker can also run the harness in a local Docker container per execution
(`ORBIT_EXECUTION_DRIVER=docker`).

## Building a custom worker image

A worker image only needs the `orbit-worker` binary and a harness CLI. The
`orbit-mcp-broker` binary is no longer required separately — it is now a
subcommand of `orbit-worker` (`orbit-worker mcp-broker`).

### Minimal Dockerfile

```dockerfile
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates git \
    && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 orbit \
    && useradd --uid 10001 --gid 10001 --no-create-home --shell /usr/sbin/nologin orbit

# Copy the single worker binary (download from GitHub releases or build from source).
COPY orbit-worker /usr/local/bin/orbit-worker
RUN chmod 0755 /usr/local/bin/orbit-worker

# Add your harness CLI here, e.g.:
# RUN npm install -g @earendil-works/pi-coding-agent@1.5.2
# or: RUN curl -fsSL ... -o /usr/local/bin/opencode && chmod 0755 /usr/local/bin/opencode

USER 10001:10001
ENV ORBIT_WORKSPACE_ROOT=/tmp/orbit-workspaces
ENTRYPOINT ["/usr/local/bin/orbit-worker"]
```

### Required environment variables

| Variable | Required | Description |
|---|---|---|
| `ORBIT_API_URL` | yes | Control plane URL (e.g. `http://orbit-api:8080`) |
| `ORBIT_WORKER_TOKEN` | yes | Worker auth token (issue via `POST /api/v1/worker-tokens` or a worker profile) |
| `ORBIT_WORKER_HARNESSES` | yes | Comma-separated harness kinds this worker claims (`mock`, `opencode`, `pi`) |
| `ORBIT_WORKER_TARGET` | yes | `local`, `docker`, or `kubernetes` (must match agent placement) |
| `ORBIT_WORKSPACE_ROOT` | no | Writable scratch dir (default `/tmp/orbit-workspaces`) |
| `ORBIT_WORKER_POLL_MS` | no | Poll interval in ms (default `1000`) |
| `ORBIT_WORKER_CONCURRENCY` | no | Max concurrent executions (default `4`) |
| `ORBIT_WORKER_PROFILE_ID` | no | Load a worker profile (agents, skills, auth) |

### Obtaining the binary

Pre-built binaries are attached to every GitHub release:

- `orbit-worker-linux-amd64` — x86_64 Linux
- `orbit-worker-linux-arm64` — ARM64 Linux

Or build from source:

```bash
cargo build --release -p orbit-worker --bin orbit-worker
# binary: target/release/orbit-worker
```

### Verifying connectivity

After starting the worker, check `GET /api/v1/workers` — the worker should appear
with `live: true` within 30 seconds. Executions with matching harness and placement
will be claimed automatically.

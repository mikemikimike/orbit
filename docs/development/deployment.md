# Deployment (Docker Compose & Kubernetes)

## Images

| Image | Dockerfile | Notes |
|---|---|---|
| `ghcr.io/orbit-platform/orbit-api` | `deploy/docker/api.Dockerfile` | multi-stage, non-root, embedded migrations |
| `ghcr.io/orbit-platform/orbit-worker` | `deploy/docker/worker.Dockerfile` | non-root, runtime adapters |
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

The chart is tailored to the mkd.local / microk8s environment used for this project:

- private registry `192.168.1.201:5000` (HTTP, LAN-only; Docker may trust it as
  `registry.mdk.local:5000`, which resolves to the same host)
- images built for `linux/amd64`
- namespace `orbit`, ingress host `orbit.mdk.local`, ingress class `nginx`

```bash
# 1. Build and push images (native arm64 dev machine → amd64 cluster)
TAG=0.1.0 ./scripts/build-images.sh
PUSH_REGISTRY=registry.mdk.local:5000 TAG=0.1.0 ./scripts/push-images.sh

# 2. Deploy
KUBE_CONTEXT=microk8s NAMESPACE=orbit TAG=0.1.0 ./scripts/deploy.sh

# Equivalent explicit Helm command
helm --kube-context microk8s upgrade --install orbit deploy/helm/orbit \
  --namespace orbit --create-namespace \
  -f deploy/helm/orbit/values.yaml \
  --set imageRegistry=192.168.1.201:5000 \
  --set api.tag=0.1.0 --set worker.tag=0.1.0 --set web.tag=0.1.0
```

Ingress routes: `/docs`, `/healthz`, `/readyz` → `orbit-api`; `/api` → `orbit-web`
(the web container proxies `/api/*` to the API at runtime, injecting the dev token
server-side when dev auth is enabled and forwarding OIDC/API bearer headers otherwise);
`/` → `orbit-web`.
The chart sets SSE-friendly nginx annotations (`proxy-buffering: off`, long
read/send timeouts). Point `orbit.mdk.local` at the ingress address (or add an
`/etc/hosts`/split-DNS entry).

Validation status: the chart has been deployed to the microk8s cluster
(`http://orbit.mdk.local`, ingress address `192.168.1.200`) and smoke-tested end-to-end
(ingress → web proxy → API → Postgres → worker → mock runtime → SSE). It was also
deployed to a local test cluster.

### macOS "no route to host" while the cluster is up

If `curl https://192.168.1.6:16443/version` works but `kubectl` fails with
`dial tcp 192.168.1.6:16443: connect: no route to host`, macOS Local Network privacy is
likely blocking the `kubectl` binary (unsigned/third-party binaries can be denied even
when the terminal app is allowed). Fix it in **System Settings → Privacy & Security →
Local Network** by enabling your terminal and `kubectl`, or run the deployment from an
allowed terminal. As a temporary workaround, forward the API through a loopback relay
started by an allowed interpreter (`127.0.0.1:16444` → `192.168.1.6:16443`) and use a
kubeconfig whose server is `https://127.0.0.1:16444`; kubectl then only needs loopback.

Key values:
- `externalDatabase.enabled` / `postgresql.enabled` (bundled Bitnami dependency is
  opt-in; production should use an external managed database).
- `externalNats.enabled` / `nats.enabled`.
- `oidc.*` — issuer, client id, scopes, and either an existing secret or plain values
  for a test cluster.
- `secrets.existingSecret` — chart does not manage secret material in production;
  `ORBIT_MASTER_KEY`, `ORBIT_DATABASE_URL`, OIDC client secret live there.
- `runtimeWorker.*` — replicas, resources, runtime allowlist, isolation settings.
- `ingress.*` — nginx annotations include SSE buffering disabled.

The chart requires no cluster-admin permissions and creates a namespaced ServiceAccount.

## OpenCode runtime worker

OpenCode runs in a separate execution-plane image (`deploy/docker/worker-opencode.Dockerfile`)
so the control plane and base worker do not bundle runtime frameworks. The worker claims
only `runtime_kind=opencode` executions (`ORBIT_WORKER_RUNTIMES=opencode`).

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
OpenCode v2.2.5. Rotate credentials by recreating the Secret and restarting the
Deployment. For local Docker/OrbStack, mount the exported auth file similarly or run the
worker with `ORBIT_WORKER_RUNTIMES=opencode` and a writable `HOME`.

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

## Kubernetes runtime execution

Phase 0 runs the mock runtime in the worker. Phase 1 adds the container/Job dispatcher:
the worker creates a per-execution Job in its own namespace using a ServiceAccount with
only `pods`/`jobs` permissions in that namespace, with limits and a TTL for cleanup.

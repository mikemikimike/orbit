# ORBIT Helm chart

Deploys the ORBIT control plane (API, web), the execution plane (mock worker, optional
OpenCode/Pi workers), and optionally PostgreSQL, NATS and the worker launcher.

- Chart: `deploy/helm/orbit`
- Default image registry: `ghcr.io/opsifyhq/orbit` (images published by CI)
- Default ingress host: `orbit.example.com`

## Prerequisites

- Kubernetes 1.27+ and `kubectl`
- Helm 3.12+
- A worker token (issued by a running API) — **required**, the chart will not render
  without it

## 1. Get the chart

```bash
git clone https://github.com/opsifyhq/orbit.git
cd orbit
helm lint deploy/helm/orbit --set worker.token=lint
```

## 2. Images

CI publishes six images per release. The chart pulls them from `imageRegistry`, so the
registry value must include the repository segment.

For release `0.7.0` with the default `imageRegistry: ghcr.io/opsifyhq/orbit`:

| Component | Reference |
|---|---|
| API | `ghcr.io/opsifyhq/orbit/orbit-api:0.7.0` |
| Web | `ghcr.io/opsifyhq/orbit/orbit-web:0.7.0` |
| Worker (mock) | `ghcr.io/opsifyhq/orbit/orbit-worker:0.7.0` |
| OpenCode worker | `ghcr.io/opsifyhq/orbit/orbit-worker-opencode:0.7.0` |
| Pi worker | `ghcr.io/opsifyhq/orbit/orbit-worker-pi:0.7.0` |
| Launcher | `ghcr.io/opsifyhq/orbit/orbit-launcher:0.7.0` |

Tags default to the chart `appVersion`, so `--version 0.7.0` pulls `:0.7.0`. Override per
component with `--set api.tag=…`, `worker.tag`, `web.tag`, `launcher.tag`,
`opencodeWorker.tag`, `piWorker.tag`.

Check what the chart will pull:

```bash
helm template orbit deploy/helm/orbit -n orbit --set worker.token=lint \
  | grep 'image:'
```

### Private registry / mirror

```bash
REGISTRY=registry.example.com:5000 TAG=0.7.0 ./scripts/build-images.sh
PUSH_REGISTRY=registry.example.com:5000 TAG=0.7.0 ./scripts/push-images.sh

helm upgrade --install orbit deploy/helm/orbit -n orbit --create-namespace \
  --set imageRegistry=registry.example.com:5000 \
  --set worker.token="$WORKER_TOKEN"
```

### Private images (imagePullSecrets)

```bash
kubectl -n orbit create secret docker-registry ghcr-auth \
  --docker-server=ghcr.io --docker-username=<user> --docker-password=<token>

helm upgrade --install orbit deploy/helm/orbit -n orbit \
  --set imageRegistry=ghcr.io/opsifyhq/orbit \
  --set 'imagePullSecrets[0].name=ghcr-auth' \
  --set worker.token="$WORKER_TOKEN"
```

## 3. Issue the worker token(s)

Start the API first (or run it elsewhere), then issue scoped tokens:

```bash
export API=http://orbit.example.com        # or a port-forward to the API service

WORKER_TOKEN=$(curl -s -X POST "$API/api/v1/worker-tokens" \
  -H "Authorization: Bearer $ORBIT_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"cluster-mock","harnesses":["mock"]}' | jq -r .token)

OPENCODE_TOKEN=$(curl -s -X POST "$API/api/v1/worker-tokens" \
  -H "Authorization: Bearer $ORBIT_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"cluster-opencode","harnesses":["opencode"]}' | jq -r .token)

PI_TOKEN=$(curl -s -X POST "$API/api/v1/worker-tokens" \
  -H "Authorization: Bearer $ORBIT_TOKEN" \
  -H 'content-type: application/json' \
  -d '{"name":"cluster-pi","harnesses":["pi"]}' | jq -r .token)
```

`$ORBIT_TOKEN` is an API token (`orbit_sk_…`) or, in dev mode, the dev token. Tokens are
scoped and revocable; rotating one invalidates the workers using the old value.

> Tip: `scripts/deploy.sh` automates the whole flow and reuses the tokens already stored
> in the cluster secret.

## 4. Install

Minimal install (bundled PostgreSQL/NATS, dev auth, mock worker):

```bash
helm upgrade --install orbit deploy/helm/orbit \
  --namespace orbit --create-namespace \
  --set worker.token="$WORKER_TOKEN" \
  --set ingress.host=orbit.example.com
```

With the OpenCode and Pi workers:

```bash
helm upgrade --install orbit deploy/helm/orbit \
  --namespace orbit --create-namespace \
  --set worker.token="$WORKER_TOKEN" \
  --set opencodeWorker.enabled=true \
  --set opencodeWorker.token="$OPENCODE_TOKEN" \
  --set opencodeWorker.auth.existingSecret=orbit-opencode-auth \
  --set piWorker.enabled=true \
  --set piWorker.token="$PI_TOKEN" \
  --set piWorker.auth.existingSecret=orbit-pi-auth \
  --set ingress.host=orbit.example.com
```

Create the harness auth secrets first:

```bash
# OpenCode: credentials exported from a logged-in install
opencode auth export > auth-export.json
kubectl -n orbit create secret generic orbit-opencode-auth \
  --from-file=auth-export.json=./auth-export.json

# Pi: the agent directory (auth.json etc.)
kubectl -n orbit create secret generic orbit-pi-auth \
  --from-file=auth.json="$HOME/.pi/agent/auth.json"
```

Verify:

```bash
kubectl -n orbit get pods,svc,ingress
kubectl -n orbit logs deploy/orbit-api --tail=50
curl -H "Authorization: Bearer $ORBIT_TOKEN" http://orbit.example.com/api/v1/healthz
```

## Required values

| Value | Required when |
|---|---|
| `worker.token` | always |
| `opencodeWorker.token` | `opencodeWorker.enabled=true` |
| `piWorker.token` | `piWorker.enabled=true` |
| `launcher.token` | `launcher.enabled=true` |
| `secrets.masterKey` | `secrets.create=true` (default) |

`helm template`/`helm lint` fail without them by design, so a worker can never start
without a scoped credential. For linting use `--set worker.token=lint`.

## Values reference

### Images

| Key | Default | Notes |
|---|---|---|
| `imageRegistry` | `ghcr.io/opsifyhq/orbit` | must include the repo segment |
| `imagePullPolicy` | `IfNotPresent` | |
| `imagePullSecrets` | `[]` | registry credentials |
| `api.image` / `api.tag` | `orbit-api` / appVersion | |
| `worker.image` / `worker.tag` | `orbit-worker` / appVersion | |
| `web.image` / `web.tag` | `orbit-web` / appVersion | |
| `launcher.image` / `launcher.tag` | `orbit-launcher` / appVersion | |

### Control plane

| Key | Default | Notes |
|---|---|---|
| `api.replicas` | `1` | stateless; scale freely |
| `api.port` | `8080` | |
| `web.replicas` | `1` | |
| `web.port` | `3000` | |
| `platform.environment` | `development` | `production` forbids dev auth |
| `platform.publicBaseUrl` | `http://orbit.example.com` | used in OIDC redirects |
| `platform.corsOrigins` | `http://orbit.example.com` | |
| `platform.seedDemo` | `true` | demo agents on first start |
| `platform.logFormat` | `json` | `text` for local |
| `platform.bodyLimitBytes` | `2097152` | |
| `platform.archiveAfterDays` | `30` | archive terminal runs older than N days; `0` disables |

### Database and bus

| Key | Default | Notes |
|---|---|---|
| `postgresql.enabled` | `true` | bundled single-replica PostgreSQL |
| `postgresql.storage` | `5Gi` | |
| `postgresql.storageClassName` | `""` | cluster default |
| `externalDatabase.enabled` / `url` | `false` / `""` | required when enabled |
| `nats.enabled` | `true` | bundled NATS JetStream |
| `nats.storage` | `2Gi` | |
| `externalNats.enabled` / `url` | `false` / `""` | required when enabled |
| `secrets.databaseUrl` | `""` | overrides the computed URL |
| `secrets.create` / `secrets.existingSecret` | `true` / `""` | bring your own Secret in production |
| `secrets.masterKey` | dev key | base64 32-byte AES key |

### Workers

| Key | Default | Notes |
|---|---|---|
| `worker.replicas` | `1` | |
| `worker.concurrency` | `4` | concurrent executions per worker |
| `worker.harnesses` | `mock` | comma-separated harness kinds |
| `worker.pollIntervalMs` | `500` | |
| `worker.token` | `""` | required |
| `worker.workspace.sizeLimit` | `1Gi` | |
| `opencodeWorker.enabled` | `false` | OpenCode CLI image |
| `opencodeWorker.auth.existingSecret` | `""` | `opencode auth export` Secret |
| `piWorker.enabled` | `false` | Pi CLI image |
| `piWorker.auth.existingSecret` | `""` | Pi agent directory Secret |

### Authentication

| Key | Default | Notes |
|---|---|---|
| `auth.mode` | `dev` | `dev` or `oidc` |
| `auth.dev.allowAnonymous` | `true` | omit tokens (dev only) |
| `auth.dev.token` | `orbit-dev-token` | injected server-side by the web proxy |
| `auth.oidc.issuer` | `""` | |
| `auth.oidc.audience` | `""` | |
| `auth.oidc.clientId` | audience | web login client |
| `auth.oidc.redirectUri` | `<publicBaseUrl>/auth/callback` | |
| `auth.oidc.jwksUrl` | `""` | |
| `auth.oidc.scopes` | `openid profile email` | |
| `auth.oidc.autoProvision` | `true` | create users on first login |
| `auth.oidc.defaultRole` | `viewer` | |
| `secrets.oidcClientSecret` | `""` | confidential clients only |

OIDC can also be configured at runtime under **Settings → OIDC / SSO**; stored values are
applied on the next API start.

### Networking and security

| Key | Default | Notes |
|---|---|---|
| `ingress.enabled` | `true` | |
| `ingress.ingressClassName` | `nginx` | |
| `ingress.host` | `orbit.example.com` | |
| `ingress.tls.enabled` / `secretName` | `false` / `orbit-tls` | |
| `caCertificates.enabled` / `secretName` / `key` | `false` / `""` / `ca-bundle.crt` | private CA bundle mounted into every workload |
| `podSecurityContext` / `containerSecurityContext` | non-root | |
| `serviceAccount.create` | `true` | namespaced, no cluster-admin |

### Launcher (optional)

| Key | Default | Notes |
|---|---|---|
| `launcher.enabled` | `false` | enables Launch/Stop/Logs in the UI |
| `launcher.token` | `""` | required when enabled |
| `launcher.imagePullSecret` | `""` | registry secret for launched pods |

The launcher gets namespaced RBAC for Pods/Jobs only. The API never receives cluster
credentials. Long-lived profiles run as Pods; single-execution launches run as Jobs with a
300s TTL.

### Other

| Key | Default |
|---|---|
| `nameOverride` / `fullnameOverride` | `""` |
| `podAnnotations` | `{}` |
| `extraEnv` | `[]` (list of env objects for all backend workloads) |
| `nodeSelector` / `tolerations` / `affinity` | `{}` / `[]` / `{}` |
| `terminationGracePeriodSeconds` | `30` |

## Production

Start from the production example (external PostgreSQL/NATS, OIDC, TLS, externally
managed Secret):

```bash
helm upgrade --install orbit deploy/helm/orbit -n orbit -f deploy/helm/orbit/values-prod.example.yaml \
  --set worker.token="$WORKER_TOKEN"
```

Checklist:

- `platform.environment=production` **and** `auth.mode=oidc` (the API refuses dev auth in
  production).
- `secrets.create=false` with `secrets.existingSecret` holding `database-url`,
  `master-key`, worker tokens and the OIDC client secret.
- `externalDatabase.enabled=true` + `externalDatabase.url`, and/or
  `externalNats.enabled=true` + `externalNats.url`.
- `ingress.tls.enabled=true` with a real certificate Secret.
- `nodeSelector`/`tolerations`/`affinity` for your node pools.

## Upgrade and uninstall

```bash
helm upgrade orbit deploy/helm/orbit -n orbit -f my-values.yaml
helm history orbit -n orbit
helm rollback orbit 1 -n orbit

helm uninstall orbit -n orbit
# bundled data lives in PVCs; delete them explicitly if you want a clean slate
kubectl -n orbit delete pvc -l app.kubernetes.io/instance=orbit
```

Database migrations run automatically when the API starts (`ORBIT_DB_AUTO_MIGRATE`), so a
newer chart upgrades the schema on rollout. Migrations are append-only.

## Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `ImagePullBackOff` | `imageRegistry` missing the repository segment (must be `ghcr.io/opsifyhq/orbit`), or private images without `imagePullSecrets` |
| `worker.token is required` at render time | issue a token and pass `--set worker.token=…` |
| API exits with `ORBIT_AUTH_MODE=dev is not allowed in production` | set `platform.environment=development`, or configure OIDC |
| Worker logs `ORBIT_WORKER_TOKEN is required` | missing/empty token Secret key |
| `launcher_unavailable` in the UI | `launcher.enabled=false` or token mismatch |
| Executions stay `queued` | no live worker for that harness/placement — check `GET /api/v1/workers` |
| SSE stalls behind a proxy | keep the chart's nginx annotations (`proxy-buffering: off`) |
| Harness auth failures | the auth Secret is missing or the export is stale; recreate it and restart the worker |

## Notes

- Images are `linux/amd64`.
- PostgreSQL/NATS bundled in this chart are single-replica and intended for test clusters.
- The chart creates no cluster-scoped resources.

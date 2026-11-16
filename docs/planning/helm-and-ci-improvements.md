# Plan: Helm chart README, image references, and CI speedups

Status: mostly implemented. Superseded for worker-token handling by
`docs/planning/helm-worker-token-bootstrap.md` (workers now launch from profiles by
default; see the CHANGELOG).
Scope: chart packaging/docs and `.github/workflows/ci.yml`

## Findings

### Image paths CI actually publishes

`.github/workflows/ci.yml` sets:

```yaml
env:
  REGISTRY: ghcr.io
  IMAGE_PREFIX: ${{ github.repository }}   # opsifyhq/orbit
```

and builds `images: ${{ env.REGISTRY }}/${{ env.IMAGE_PREFIX }}/${{ matrix.image }}`.

Published references are therefore:

```
ghcr.io/opsifyhq/orbit/orbit-api:0.7.0
ghcr.io/opsifyhq/orbit/orbit-worker:0.7.0
ghcr.io/opsifyhq/orbit/orbit-web:0.7.0
ghcr.io/opsifyhq/orbit/orbit-launcher:0.7.0
ghcr.io/opsifyhq/orbit/orbit-worker-opencode:0.7.0
ghcr.io/opsifyhq/orbit/orbit-worker-pi:0.7.0
```

Not `ghcr.io/opsifyhq/orbit-api:0.7.0`. GHCR uses the repository path as the
namespace, then the image name.

### Why this is unclear today

- `deploy/helm/orbit/values.yaml` defaults `imageRegistry: registry.example.com:5000`,
  a placeholder that points at no real registry.
- The chart composes `{{ imageRegistry }}/{{ image }}:{{ tag }}`, so the registry value
  must include the repository segment (`ghcr.io/opsifyhq/orbit`).

Verified with `helm template`:

| `imageRegistry` | rendered |
|---|---|
| `ghcr.io/opsifyhq/orbit` | `ghcr.io/opsifyhq/orbit/orbit-api:0.7.0` ✅ |
| `ghcr.io/opsifyhq` | `ghcr.io/opsifyhq/orbit-api:0.7.0` ❌ |

Tags already default to `.Chart.AppVersion`, so `--version 0.7.0` → `:0.7.0`.

## Plan 1 — Make the chart work out of the box

1. `deploy/helm/orbit/values.yaml`
   - `imageRegistry: ghcr.io/opsifyhq/orbit` as the default.
   - Comment explaining the shape: "must include the repository segment; e.g.
     `ghcr.io/opsifyhq/orbit` renders `ghcr.io/opsifyhq/orbit/orbit-api`".
2. `deploy/helm/orbit/values-prod.example.yaml`
   - Keep an explicit private-mirror example with a comment on mirroring.
3. Document `imagePullSecrets` for private GHCR (or any private registry).

Effort: ~20 minutes.

## Plan 2 — Add `deploy/helm/orbit/README.md`

Contents:

1. **Prerequisites** — Helm ≥ 3.12, kubectl, a Kubernetes cluster.
2. **Get the chart**
   ```bash
   git clone https://github.com/opsifyhq/orbit.git
   cd orbit
   ```
3. **Install with published images (default)**
   ```bash
   helm upgrade --install orbit deploy/helm/orbit \
     --namespace orbit --create-namespace \
     --set worker.token="$WORKER_TOKEN"
   ```
   Note: `worker.token` is required; issue it with `POST /api/v1/worker-tokens` from a
   running API (or `scripts/deploy.sh`, which reuses existing tokens).
4. **Install from a private mirror**
   ```bash
   REGISTRY=registry.example.com:5000 TAG=0.7.0 ./scripts/build-images.sh
   PUSH_REGISTRY=registry.example.com:5000 TAG=0.7.0 ./scripts/push-images.sh
   helm upgrade --install orbit deploy/helm/orbit -n orbit --create-namespace \
     --set imageRegistry=registry.example.com:5000 \
     --set worker.token="$WORKER_TOKEN"
   ```
5. **Image reference table** — the six images, their Dockerfiles, and the exact pulled
   reference for a given `imageRegistry` + `appVersion`.
6. **Required values**
   | Value | When |
   |---|---|
   | `worker.token` | always |
   | `opencodeWorker.token` | `opencodeWorker.enabled=true` |
   | `piWorker.token` | `piWorker.enabled=true` |
   | `launcher.token` | `launcher.enabled=true` |
   | `secrets.masterKey` | `secrets.create=true` (default) |
7. **Values reference** — every top-level key with default and purpose.
8. **Authentication** — `dev` vs `oidc`; note that the API refuses to start with
   `platform.environment=production` and `auth.mode=dev`.
9. **Ingress, TLS, custom CA** — `ingress.*`, `ingress.tls.*`, `caCertificates.*`.
10. **Harness workers** — enabling `opencodeWorker` / `piWorker`, auth Secret shape,
    what each image bundles.
11. **Launcher** — `launcher.enabled`, namespace-scoped RBAC, Pod vs Job behaviour.
12. **Operations** — upgrade, uninstall, `helm lint`, `helm template` with
    `--set worker.token=lint`, verifying rendered tags.
13. **Troubleshooting**
    - `ImagePullBackOff` → wrong registry path (needs the repo segment) or private repo
      without `imagePullSecrets`.
    - `worker.token is required` → a fixed worker is enabled without a token
      (`worker.enabled=true`).
    - API crash `ORBIT_AUTH_MODE=dev is not allowed in production` → set
      `platform.environment=development` or configure OIDC.
    - `launcher_unavailable` → launcher not enabled/configured.

Effort: ~1 hour.

## Plan 3 — Speed up CI

Baseline: wall clock is dominated by the Rust image builds; each of `api`, `worker` and
`launcher` recompiles the whole workspace from scratch inside Docker, then the harness
images build after them.

| # | Change | Why | Expected saving |
|---|---|---|---|
| 1 | Dependency-caching layer in `api`/`worker`/`launcher` Dockerfiles (copy manifests, dummy `src`, build deps, then copy real sources and rebuild only the workspace crates) | Today any source change recompiles ~370 crates | 5–8 min per image on repeat builds |
| 2 | Build `api`+`worker`+`launcher` in one job via `docker/bake-action` with a shared GHA cache | Three jobs compile identical dependencies | 3–5 min wall clock |
| 3 | Run the test suite once with `ORBIT_TEST_DATABASE_URL` instead of `cargo test` twice | Duplicate full run today | 2–3 min |
| 4 | Cache the pnpm store in the web job | Cold install each run | 30–60 s |
| 5 | Build `harness-images` only on `v*` tags | They depend on the base worker and add a serial stage | 2–3 min on `main` |
| 6 | `concurrency:` group per ref with `cancel-in-progress: true` | Superseded pushes keep runners busy | avoids queueing |
| 7 | Optional: `sccache` for the host cargo job | Only helps the rust test job | 1–2 min |

Expectations:
- First build of a commit: ~8–12 min (sources changed).
- Docs-only or repeat builds: ~4–6 min.
- No change to security scanning or `linux/amd64`-only builds.

Implementation notes:
- Dockerfile change keeps multi-stage builds and `--locked`.
- Bake builds need distinct `context`/`dockerfile` targets; the same cache scope can be
  shared because the dependency layer is identical.
- `concurrency` must not cancel tag builds: use
  `group: ci-${{ github.workflow }}-${{ github.ref }}` and
  `cancel-in-progress: ${{ !startsWith(github.ref, 'refs/tags/') }}`.

Effort: ~2–3 hours including verification.

## Order of work

1. Plan 1 (chart default + comment) — smallest, removes the confusion.
2. Plan 2 (chart README).
3. Plan 3 (CI), measured before/after with the same commit so numbers are comparable.

## Open question

Should the chart default to GHCR (`ghcr.io/opsifyhq/orbit`) or stay generic and require
`--set imageRegistry=…`? Defaulting to GHCR makes `helm install` work immediately but
assumes public packages; a generic default keeps the chart neutral.

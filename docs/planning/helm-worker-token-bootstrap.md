# Helm worker-token bootstrap: problem and fix

Status: temporary notes (safe to delete once the changes land)
Related: `deploy/helm/orbit/README.md`, `deploy/helm/orbit/templates/secret.yaml`,
`scripts/deploy.sh`, `services/worker/src/main.rs`

## The confusion

The Helm README lists "3. Issue the worker token(s)" **before** "4. Install", which
reads like a follow-up step. It is not: it is a prerequisite. There is **one**
worker token, not a bootstrap token plus a later API-issued token.

The chart hard-fails without `worker.token` because the worker is DB-less: on boot it
calls `GET /api/v1/worker/session` with the token, retries for 90s, then exits. The
token must already exist (hashed) in PostgreSQL, so a worker can never bootstrap
itself. Meanwhile the API that issues the token is what Helm installs → real
chicken-and-egg.

## Actual process today

```text
1. API running (control plane only)
2. authenticate to API
     dev mode:  orbit-dev-token
     oidc:      log in to web → Settings → create API token orbit_sk_…
3. POST /api/v1/worker-tokens  {"name":"cluster-mock","harnesses":["mock"]}
     → orbit_wk_… (returned once, stored hashed, scoped, revocable)
4. helm --set worker.token=orbit_wk_…
     worker starts, validates token, claims executions
```

## Workaround with the chart as-is (no code changes)

```bash
# phase 1: control plane only (worker scaled to zero, throwaway token value)
helm upgrade --install orbit deploy/helm/orbit -n orbit --create-namespace \
  --set worker.replicas=0 --set worker.token=bootstrap \
  --set ingress.host=orbit.mdk.local

# phase 2: issue the real token
kubectl -n orbit port-forward svc/orbit-api 8080:8080 &
TOKEN=$(curl -s -X POST localhost:8080/api/v1/worker-tokens \
  -H 'Authorization: Bearer orbit-dev-token' -H 'content-type: application/json' \
  -d '{"name":"cluster-mock","harnesses":["mock"]}' | jq -r .token)

# phase 3: hand it to Helm
helm upgrade orbit deploy/helm/orbit -n orbit --reuse-values \
  --set worker.token="$TOKEN" --set worker.replicas=1
```

## Defects that make even this fragile

1. **No checksum annotation on worker pods.** A `helm upgrade` that only changes the
   token does not restart workers; they keep the old token until
   `kubectl rollout restart deploy/orbit-worker`. Rotation silently desyncs old and new
   pods.
2. **`scripts/deploy.sh` does not do what the docs claim.** It only *reuses*
   `worker-token` from an existing `orbit-secrets`; on a fresh install it passes
   nothing, so Helm fails. It never issues a token. README (~line 127) and
   `templates/NOTES.txt` say it automates the flow — the reuse half is true, the
   automation half is not. Violates the "honesty about integrations" rule.
3. **`secrets.existingSecret` fails late.** With an external secret, the chart never
   templates `worker-token`, so a missing key surfaces at pod mount (volume/env
   failure) instead of at render time.
4. **Production ownership is unclear.** For real deployments Helm should not own
   tokens: `secrets.create=false`, `secrets.existingSecret=<managed secret>` with a
   `worker-token` key from Vault/External Secrets, reissued via the API on rotation.

## Planned changes

1. **`worker.enabled: true`** (and `opencodeWorker.enabled` / `piWorker.enabled`):
   when false, skip the Deployment and make `worker.token` optional in
   `templates/secret.yaml`. Phase 1 becomes `--set worker.enabled=false` with no
   throwaway token.
2. **Checksum annotations** on the worker, opencode-worker, pi-worker and launcher pod
   templates, e.g.
   `checksum/config: {{ include (print $.Template.BasePath "/secret.yaml") . | sha256sum }}`,
   so token changes roll pods.
3. **`scripts/deploy.sh`**: if no token exists in the cluster secret and none is passed
   on the command line, deploy the control plane only, port-forward the API, issue the
   scoped tokens, print them, then upgrade. Otherwise just reuse the existing tokens.
4. **Docs**: reorder the Helm README so token issuance is an explicit prerequisite
   step, and correct the `deploy.sh` claim in README + `NOTES.txt`.
5. **Optional (needs a decision record)**: `ORBIT_WORKER_TOKEN_BOOTSTRAP` on the API —
   a hashed, revocable, startup-only token so one `helm install` can come up fully
   wired. Bigger design change; record it under `docs/decisions/` if adopted.

Effort: items 1–4 are contained (~2–3h including `helm template`/lint checks and a
microk8s smoke test). Item 5 is a separate design discussion.

## Implementation status (Option 2 adopted; Option 3 rejected)

Option 2 is implemented: platform workers run through the launcher by default, the chart
no longer ships a static worker unless `worker.enabled=true`, `launcher.token` is
generated and reused from the release Secret, and the API seeds one launchable mock
profile when a launcher is configured. Option 3 (ServiceAccount workload identity) is
rejected because workers also run on Docker/local hosts where no ServiceAccount exists;
custom workers keep using API-issued `orbit_wk_…` tokens.

## Can the operator create a Secret with the token and have the API use it?

Yes, but not with today's auth path — the Secret is only half the wiring.

What already works:

- The worker side is free: `worker.token` only renders into `orbit-secrets` as the
  `worker-token` key, and the Deployment reads it via `secretKeyRef`. Point
  `secrets.existingSecret` at an operator-managed Secret with that key and the chart
  uses it as-is.
- The launcher/opencode/pi worker tokens work the same way
  (`opencode-worker-token`, `pi-worker-token`, `launcher-token`).

What does not work:

- The API never reads token plaintext from a Secret. `crates/orbit-api/src/worker_auth.rs`
  hashes the presented bearer token and looks it up in `worker_tokens`
  (`token_hash`, `revoked_at`, `expires_at`). A plaintext token sitting in a Kubernetes
  Secret is unknown to the API until something writes its SHA-256 hash into the target
  database. `hash_api_token` produces `sha256:<hex>`; `token_prefix` is the first 16
  characters; tokens are `orbit_wk_<base64url>`.

### Options

| Option | Works today? | Notes |
|---|---|---|
| A. API imports a mounted Secret at startup | needs code | Read token files, `hash_api_token`, insert if absent, revoke same-name stale hashes. One `kubectl create secret` + `helm install`; no token in Helm values, no API call. |
| B. Self-verifying signed worker tokens (HMAC with master key) | needs a bigger change | No DB row needed for auth, but revocation needs a deny-list and the token format plus `worker_auth.rs` change. Conflicts with "durable state lives in PostgreSQL". |
| C. Seed the DB directly (one-off Job or SQL) | yes, zero code | Bypasses audit and revocation UX. Interim only. |

### Option A — recommended design

```yaml
# operator creates this before `helm install`
apiVersion: v1
kind: Secret
metadata:
  name: orbit-worker-auth
  namespace: orbit
stringData:
  worker.token:          orbit_wk_<random>   # -> harnesses ["mock"]
  opencode-worker.token: orbit_wk_<random>   # -> harnesses ["opencode"]
  pi-worker.token:       orbit_wk_<random>   # -> harnesses ["pi"]
```

1. Chart
   - `workerBootstrap.existingSecret` (optional): mounts that Secret read-only into
     `orbit-api` at `/run/orbit/bootstrap/`.
   - When set, do not render token values into `orbit-secrets` (the chart currently
     `required`s them in `templates/secret.yaml`); `worker.token` etc. become optional.
   - Worker pods mount the same Secret, so there is one source of truth.
2. API startup (`crates/orbit-api/src/bootstrap.rs::initialize`, after migration and
   default-org resolution)
   - For each `*.token` file: hash with `hash_api_token`, then
     `INSERT ... ON CONFLICT (token_hash) DO NOTHING` via a new
     `Store::ensure_bootstrap_worker_token`.
   - Log name and prefix only; never the plaintext.
   - Org scope: `ORBIT_BOOTSTRAP_ORG_SLUG`, defaulting to `first_organization()`
     (ambiguous in multi-org OIDC setups, so make it explicit there).
3. Rotation and removal
   - Add `source text NOT NULL DEFAULT 'api'` to `worker_tokens` (new append-only
     migration).
   - On startup, revoke `source='bootstrap'` rows for a given name whose hash differs
     from the current file, and revoke names whose file disappeared. Removing the
     Secret entry or key revokes that token on the next API restart/upgrade.

### Two additional wins

1. **The token leaves Helm state.** Today `--set worker.token=…` stores the plaintext in
   the Helm release Secret (`sh.helm.release.v1.orbit.vN`), readable by anyone with
   Secret read access in the namespace, across every revision. An operator-managed
   Secret removes that.
2. **No first-login bootstrap problem.** Production OIDC requires an API token
   (`orbit_sk_…`) created in Settings before you can call
   `POST /api/v1/worker-tokens` — itself a chicken-and-egg on a fresh cluster. The
   Secret import path removes that dependency from the install flow.

### Option C — interim SQL (no code changes)

Generate tokens however you like (`openssl rand -base64 32`, prefixed `orbit_wk_`), put
them in a Secret, and register the hashes once:

```sql
INSERT INTO worker_tokens (id, org_id, name, token_prefix, token_hash, harnesses)
SELECT gen_random_uuid(), id, 'cluster-mock', left(:'token', 16),
       'sha256:' || encode(sha256(convert_to(:'token', 'UTF8')), 'hex'), ARRAY['mock']
FROM organizations ORDER BY created_at LIMIT 1
ON CONFLICT (token_hash) DO NOTHING;
```

Then either keep using `worker.token` (which still lands in Helm state) or set
`secrets.create=false --set secrets.existingSecret=orbit-worker-auth` with the matching
`worker-token` key.

### Decision needed

Adopt Option A as the supported install path, or keep the API-issued flow and only fix
the ordering/docs plus `worker.enabled` and the checksum annotation. Option B is not
recommended.

## Should ORBIT generate tokens for its own workers?

The observation is correct: a user only needs `POST /api/v1/worker-tokens` when they add
a **custom/external** worker. The platform's own workers are ours; asking an operator to
mint a credential for them is friction that only exists because of how the chart is
wired today.

But "generate at initialization" is not sufficient on its own. The API stores only a
hash (`worker_tokens.token_hash`), and the plaintext is returned exactly once to the
caller. That is deliberate ("secrets are write-only"). So an API-generated token has no
way to reach a worker pod unless the caller that generated it is also the caller that
deploys the worker. There are only three viable distribution channels:

| Channel | How the platform worker gets the token | Already exists? |
|---|---|---|
| Pre-shared Secret (chart or operator) | Operator/chart writes the plaintext into a Kubernetes Secret; API imports the hash at startup (see Option A above) | No, needs the import code |
| Platform-mediated launch | The API mints the token at launch time and hands it to the launcher, which injects `ORBIT_WORKER_TOKEN` into the Pod/Job | **Yes** — `worker_launches.rs` + `issue_profile_token`, `services/launcher` |
| Workload identity | Worker presents a ServiceAccount JWT; API validates it via TokenReview and maps SA + namespace to an org/profile. No token material at all | No, new ADR |

### What the existing code already proves

- `POST /api/v1/worker-profiles/{id}/bootstrap` and the Launch flow already mint a
  scoped, per-profile token with `rotate=true`, pass the plaintext to the launcher, and
  revoke it when the launch is stopped (`worker_launches.rs` revokes
  `worker_token_id` on stop). That is exactly "the platform issues its own worker
  credentials" — it is just not used by the Helm chart.
- `launcher.token` is a separate, operator-set shared secret between API and launcher;
  the launcher is the only component with workload RBAC. The API never gets it.

So the split we want is:

| Worker type | Credential | User action |
|---|---|---|
| Platform worker launched by ORBIT (launcher, Pod/Job per profile) | API-minted at launch, revoked on stop | none |
| Platform worker as a static chart Deployment (mock demo worker) | auto-generated, platform-owned (chart Secret + API import, or SA identity) | none |
| Custom/external worker (own cluster, laptop, Docker) | API-issued `orbit_wk_…`, shown once, revocable in Workers/Settings | create token, copy it |

### Proposed sequence

1. **Now (small):** `worker.enabled` + `worker.token` optional in the chart; token
   import from a mounted Secret (Option A). The demo worker's token is generated by the
   chart/operator once and never surfaces in the UI. Custom workers keep the current API
   flow.
2. **Next (recommended direction):** make the launcher/profile path the default way to
   run platform workers. The chart stops shipping static worker Deployments; operators
   who want the mock/demo worker use a seeded profile, or `worker.enabled=true` with the
   generated token. Removes the token from Helm values entirely.
3. **Later (ADR):** for in-cluster workers, accept a projected ServiceAccount token
   (`TokenReview` + SA-name/namespace mapping) so there is no shared worker secret at
   all. Costs: API needs `TokenReview` permission (not workload RBAC), and it only works
   in Kubernetes — Docker/local workers still use `orbit_wk_…`.

### Token records for platform workers

- New records use a distinguishable name and source, e.g. `name='platform:mock'`,
  `source='platform'` (migration adds the column).
- Hide them from the user-facing token list, or show them read-only as "managed by
  ORBIT" with Revoke meaning "regenerate on next API start / launch".
- Same scoping rules apply: one harness per token, revocable, `last_used_at` still
  recorded.
- Never log or expose the plaintext; nothing changes about `SecretLease` or the
  write-only secret rule.

### Do not do

- Do not make tokens self-verifying with the master key for platform workers: it looks
  convenient (no DB row, no distribution), but rotation and revocation then depend on
  the master key and a deny-list, which is worse than the current model.
- Do not have the API write worker Deployments or Secrets: `orbit-api` must stay free of
  workload RBAC. Distribution belongs to the launcher or to pre-created Secrets.

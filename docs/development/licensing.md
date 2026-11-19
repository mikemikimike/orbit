# Offline licensing

The design is in [`docs/decisions/0010-offline-licensing.md`](../decisions/0010-offline-licensing.md).
This page is the operator quickstart.

## Install a license (airgapped)

The license is an Ed25519-signed JWS token. No installation ID, activation call or
network access is required.

```bash
kubectl -n orbit create secret generic orbit-license \
  --from-file=license.jwt=/path/to/orbit-license.jwt

helm upgrade orbit deploy/helm/orbit -n orbit --reuse-values \
  --set license.existingSecret=orbit-license \
  --set license.key=license.jwt
```

The chart mounts it at `/etc/orbit-license.jwt` and sets `ORBIT_LICENSE_FILE`. For
Docker/Compose mount the file and set `ORBIT_LICENSE_FILE`, or set `ORBIT_LICENSE` to the
token (inline values are not recommended for production).

## Renew

1. Replace the key in the Secret (`kubectl -n orbit create secret ... --dry-run=client -o yaml | kubectl apply -f -`).
2. Kubernetes updates the mounted file within ~60s; the API hot-reloads it. No restart
   needed when the license is delivered as a mounted file.

If the license was provided inline (`license.value` / `ORBIT_LICENSE`), restart the API
or run `helm upgrade`.

## Status

`GET /api/v1/meta` includes:

```json
"license": {
  "tier": "enterprise",
  "customer": "Acme",
  "features": ["custom_roles", "quotas"],
  "limits": { "seats": 50, "agents": 500 },
  "expiresAt": "2027-11-01T00:00:00Z",
  "daysRemaining": 350,
  "grace": false,
  "valid": true
}
```

Without a license the installation runs as `community`. An expired license keeps working
through `graceDays`, then premium writes are rejected while reads, exports, backups and
running executions continue.

## Vendor signing tool

The offline signer lives in `services/license-cli` and is not part of the runtime images:

```bash
cargo build -p orbit-license-cli

# 1. Generate a signing key once; keep key.json offline.
./target/debug/orbit-license-cli keygen --out key.json --kid orbit-2026-01

# 2. Write claims.json (v, customer, plan, features, limits, issued_at, expires_at, ...).
# 3. Sign.
./target/debug/orbit-license-cli sign --key key.json --claims claims.json --out license.jwt

# 4. Show the public key entry for the build/runtime.
./target/debug/orbit-license-cli public-key --key key.json
```

Add the printed public key to `crates/orbit-security/license-public-keys.json` as
`{"kid":"base64url"}` and rebuild. This file is public by design and is compiled into
release images. Release builds ignore `ORBIT_LICENSE_PUBLIC_KEYS`; the env var is only read
by debug builds for local testing. Never commit `key.json`.

## Local test license (debug builds only)

To exercise licensed features locally (access teams, sharing, custom roles) without the
vendor key, sign a throwaway license with your own key. This only works against a **debug**
API build, because release builds ignore `ORBIT_LICENSE_PUBLIC_KEYS`.

```bash
cargo build -p orbit-license-cli -p orbit-api-server --bin orbit-api --bin orbit-license-cli
mkdir -p /tmp/orbit-license && cd /tmp/orbit-license   # outside the repo
../path/to/target/debug/orbit-license-cli keygen --out key.json --kid orbit-local-test
cat > claims.json <<'JSON'
{ "v": 1, "id": "local-test", "customer": "Local test", "plan": "enterprise", "trial": true,
  "features": ["custom_roles", "teams", "resource_permissions"],
  "limits": { "seats": 50, "agents": 500, "workers": 25, "executions_per_day": 10000 },
  "issued_at": "2026-11-19T00:00:00Z", "expires_at": "2027-11-19T00:00:00Z" }
JSON
orbit-license-cli sign --key key.json --claims claims.json --out license.jwt

# Run the debug API with it (plus your usual ORBIT_* settings):
ORBIT_LICENSE_FILE=$PWD/license.jwt \
ORBIT_LICENSE_PUBLIC_KEYS="$(orbit-license-cli public-key --key key.json)" \
  target/debug/orbit-api
```

`GET /api/v1/license` should report `"tier": "enterprise"`. Delete `key.json` when done and
never commit it or add its public key to `license-public-keys.json`.

## Verification keys

```json
// crates/orbit-security/license-public-keys.json
{ "orbit-2026-01": "base64url-raw-ed25519-public-key" }
```

The object may contain multiple keys during rotation. An unknown `kid` is rejected.

Public keys are embedded in the API by `kid`. Development builds include a documented dev
key; production builds must add the vendor public key. `ORBIT_LICENSE_PUBLIC_KEYS` accepts
a JSON object of `kid -> base64` for testing/custom builds.

The private signing key is vendor-side only (offline/HSM) and must never be committed.

## Current gating

`settings:write` custom role creation/update/deletion and custom-role assignment require
`custom_roles`. Other #15 features (resource permissions, shared connections, quotas,
marketplace) are not implemented yet.

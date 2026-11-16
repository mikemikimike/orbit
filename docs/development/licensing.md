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

## Verification keys

Public keys are embedded in the API by `kid`. Development builds include a documented dev
key; production builds must add the vendor public key. `ORBIT_LICENSE_PUBLIC_KEYS` accepts
a JSON object of `kid -> base64` for testing/custom builds.

The private signing key is vendor-side only (offline/HSM) and must never be committed.

## Current gating

`settings:write` custom role creation/update/deletion and custom-role assignment require
`custom_roles`. Other #15 features (resource permissions, shared connections, quotas,
marketplace) are not implemented yet.

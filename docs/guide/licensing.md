# Licensing

ORBIT uses an offline, installation-wide signed license. No installation ID, activation
call or network access is required.

## Install

```bash
kubectl -n orbit create secret generic orbit-license \
  --from-file=license.jwt=/path/to/orbit-license.jwt

helm upgrade orbit deploy/helm/orbit -n orbit --reuse-values \
  --set license.existingSecret=orbit-license
```

The chart mounts it at `/etc/orbit-license.jwt`; `ORBIT_LICENSE` / `ORBIT_LICENSE_FILE`
work for Docker/Compose. The API hot-reloads mounted files (~60s).

## Tiers and limits

- Community: core agents, executions, workers, OIDC/SSO, audit, exports
- Enterprise: custom roles, resource permissions (planned), shared connections (planned),
  quotas (planned), marketplace (planned)
- Limits: seats, agents, workers, executions/day. Warn at 80%, hard cap at 100%
- 30-day grace after expiry; then premium writes are blocked while reads, exports,
  backups, logins and running executions continue

Status is shown under **Settings → License** and in `GET /api/v1/license`.

## Signing

The vendor tool is offline: `services/license-cli keygen|sign|public-key`. Public keys are
embedded by `kid` or supplied through `ORBIT_LICENSE_PUBLIC_KEYS`; the private key stays
vendor-side. Full workflow: [docs/development/licensing.md](../development/licensing.md).

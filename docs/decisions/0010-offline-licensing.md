# 0010 — Offline licensing and entitlements

Status: Proposed. Tracks the licensing part of #15.

## Constraints

- Must work fully **airgapped**: no activation call, no phone-home, no JWKS/OCSP fetch.
- Self-hosted installation must keep working when the license expires (read/export/backup),
  and must never terminate running executions because of licensing.
- Licensing is commercial gating, not a security boundary. Sandboxing still comes from
  worker isolation.

## Decision

Use a **signed offline license file** verified locally.

- Format: compact JWS/JWT, `alg = EdDSA` (Ed25519), with a `kid` header. No symmetric
  secret ever ships in the image.
- Payload (canonical JSON, versioned):

```json
{
  "v": 1,
  "licenseId": "lic_...",
  "customer": "Acme",
  "orgSlug": "acme",
  "installationId": "optional-uuid",
  "plan": "enterprise",
  "features": ["custom_roles", "shared_connections", "quotas", "marketplace"],
  "limits": { "seats": 50, "agents": 200, "executionsPerDay": 5000, "workers": 20 },
  "issuedAt": "2026-11-01T00:00:00Z",
  "expiresAt": "2027-11-01T00:00:00Z",
  "graceDays": 14
}
```

- The public verification keys are compiled into `orbit-api` (`kid` → Ed25519 public key).
  The private signing key lives offline with the vendor (HSM or offline machine); it is
  never in CI, never in the repo, never in a customer environment.
- The license is delivered as a file (`ORBIT_LICENSE_FILE`, default
  `/etc/orbit/license.jwt`) or a Helm Secret key (`license.existingSecret`, key
  `license.jwt`). It is not secret material, but the file is mounted read-only and never
  logged.
- With no license (or an invalid signature) the product runs as the free/community tier.
  Invalid/expired premium licenses fall back to community entitlements; they are never a
  startup failure.

## Airgapped workflow

1. Customer deploys ORBIT without a license.
2. The UI/API shows an **installation ID** (generated at first start, stored in
   `platform_settings`) and the org slug. This is the only value that leaves the airgap.
3. Customer sends the installation ID to the vendor (email/support portal).
4. Vendor signs a license with the offline key and returns the file.
5. Customer creates/updates the Secret and restarts the API (or Helm upgrade).
6. Renewal is the same flow. No outbound traffic from the customer environment is needed.

Installation-ID binding is optional per license (`installationId` omitted = any
installation of that customer). Binding is copy-deterrence, not security.

## Expiry and grace

- `expiresAt` + `graceDays` are evaluated with a small clock-skew allowance (5 min).
- During grace: premium features remain enabled; the UI shows a renewal banner and audit
  events record the expiry.
- After grace: premium **writes/configuration** are blocked; reads, exports, backups,
  running executions and logins keep working. No execution is cancelled.
- The license cannot reduce community permissions; it only unlocks premium entitlements.

## Entitlements in the API

- `LicenseStatus { tier, features, limits, expires_at, customer, days_remaining, grace }`
  is loaded once at startup and exposed via `/api/v1/meta` and the Settings page.
- Handler checks are explicit (`entitlements.require(Feature::CustomRoles)`), not scattered
  string checks.
- Quotas are enforced where the resource is created (agents, workers, executions) and are
  server-side; the UI only mirrors the limits.
- Workers, launcher and harness adapters do not verify the license. The API is the only
  entitlement decision point, so airgapped execution keeps working.

## Key management and rotation

- Support multiple public keys by `kid`. A new key is added to the binary before use;
  old licenses remain verifiable until they expire.
- Reject unknown `kid`, unknown `alg`, malformed payloads and unsigned files.
- Revocation in airgapped environments is not practical; use short validity windows
  (annual or quarterly renewal) instead of an online revocation list. An optional signed
  revocation file can be added later for on-prem customers who can receive files.

## Explicit non-goals

- No hardware fingerprint/MAC binding (breaks Kubernetes and DR).
- No online activation or heartbeat.
- No license data in prompts, worker events or telemetry by default.
- No attempt to protect the license check with obfuscation; signature and honest customers
  are the mechanism.

## Implementation phases

1. `orbit-security::license` verifier (Ed25519, `kid`, expiry/grace, canonical claims) +
   test fixtures with a disposable keypair.
2. API startup loads file/Secret, exposes status, seeds the community entitlements.
3. Gate the #15 features (custom roles, resource permissions, shared connections, quotas,
   marketplace) with explicit feature checks.
4. Helm `license.*` values + docs (airgapped install/renew) and a Settings license panel.
5. Vendor-side signing tool (offline CLI) that is not part of the runtime image.

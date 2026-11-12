# OIDC login (Dex / Keycloak)

ORBIT uses standard OIDC discovery, Authorization Code + PKCE and JWKS validation. No
provider-specific code is required beyond the issuer URL and client registration.

## Required configuration

| Setting | Env | Example |
|---|---|---|
| Auth mode | `ORBIT_AUTH_MODE` | `oidc` |
| Issuer | `ORBIT_OIDC_ISSUER` | `https://dex.mdk.local` |
| Audience | `ORBIT_OIDC_AUDIENCE` | `orbit` |
| Client ID | `ORBIT_OIDC_CLIENT_ID` | `orbit` |
| Client secret | `ORBIT_OIDC_CLIENT_SECRET` | optional for public PKCE clients |
| Redirect URI | `ORBIT_OIDC_REDIRECT_URI` | `http://orbit.mdk.local/auth/callback` |
| Scopes | `ORBIT_OIDC_SCOPES` | `openid profile email` |
| JWKS | `ORBIT_OIDC_JWKS_URL` | issuer keys URL |

`ORBIT_OIDC_CLIENT_ID` defaults to the audience. The redirect URI must be registered
exactly at the provider.

The API fetches:

```text
<issuer>/.well-known/openid-configuration
```

for `authorization_endpoint` and `token_endpoint`. The browser never receives the client
secret.

## Dex example

Dex static client:

```yaml
staticClients:
  - id: orbit
    name: ORBIT
    secret: optional-for-confidential-client
    redirectURIs:
      - http://orbit.mdk.local/auth/callback
```

Set:

```text
ORBIT_OIDC_ISSUER=https://dex.mdk.local
ORBIT_OIDC_AUDIENCE=orbit
ORBIT_OIDC_CLIENT_ID=orbit
ORBIT_OIDC_REDIRECT_URI=http://orbit.mdk.local/auth/callback
ORBIT_OIDC_JWKS_URL=https://dex.mdk.local/keys
```

Dex access tokens may be opaque. ORBIT uses the `id_token` from the token response as the
browser bearer token and validates it against JWKS.

## Keycloak example

Create an OpenID Connect client:

- Client ID: `orbit`
- Client authentication: off for public PKCE client, or on with a client secret
- Standard flow: enabled
- Valid redirect URI: `http://orbit.mdk.local/auth/callback`
- Web origins: `http://orbit.mdk.local`

Issuer:

```text
https://keycloak.example.com/realms/<realm>
```

Set the same five variables plus `ORBIT_OIDC_JWKS_URL` if your deployment does not expose
discovery at the issuer. ORBIT’s API validates the JWT audience against
`ORBIT_OIDC_AUDIENCE` and auto-provisions users when enabled.

## Web flow

```text
/login
  → GET /api/v1/auth/oidc/config
  → redirect to authorization_endpoint with PKCE
  → /auth/callback
  → POST /api/v1/auth/oidc/token { code, codeVerifier }
  → API exchanges code at token_endpoint
  → browser stores id_token as orbit.access_token
  → API requests use Authorization: Bearer <id_token>
```

Logout currently clears the local token. Provider end-session logout and refresh-token
rotation are not implemented yet.

## Helm

```yaml
auth:
  mode: oidc
  oidc:
    issuer: https://dex.mdk.local
    audience: orbit
    clientId: orbit
    redirectUri: http://orbit.mdk.local/auth/callback
    jwksUrl: https://dex.mdk.local/keys
    scopes: "openid profile email"
    autoProvision: true
    defaultRole: viewer
secrets:
  oidcClientSecret: ""
```

For a public PKCE client leave `oidcClientSecret` empty. For a confidential client set
it; the API adds it to the server-side token exchange.

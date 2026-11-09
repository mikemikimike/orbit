# ADR-0006: Token-based auth and SSE for live updates

**Status:** Accepted

## Context
The same backend serves a web app now and native apps later. WebSockets add complexity
for proxy/ingress configuration and reconnection logic; execution logs are append-only.

## Decision
- Authentication is bearer-token based (`Authorization: Bearer …`) for all clients.
  OIDC Authorization Code + PKCE is performed by the client; ORBIT validates JWT access
  tokens against configured JWKS and maps `(iss, sub)` to a user. API keys are hashed
  and scoped for automation.
- No cookie-only flows; no browser-specific backend behavior.
- Live execution updates use Server-Sent Events at
  `/api/v1/executions/{id}/events/stream` with `Last-Event-ID` replay.
- SSE is implemented by replaying persisted `execution_events` and polling for new rows
  (250–1000 ms), so it is correct across API replicas without sticky sessions.

## Consequences
- Ingress/Helm must disable response buffering for the stream path (documented).
- WebSocket support can be added later without changing the domain model.

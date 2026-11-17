# Outputs (webhooks, Slack, Mattermost)

Outputs deliver an execution result after it reaches a terminal state. The first slice is
API-only; the UI is tracked in #28.

## Create

```bash
curl -X POST "$API/api/v1/outputs" \
  -H "Authorization: Bearer $ORBIT_TOKEN" \
  -H 'content-type: application/json' \
  -d '{
    "name": "release-slack",
    "kind": "slack",
    "agentId": null,
    "events": ["succeeded", "failed"],
    "webhookUrl": "https://hooks.slack.com/services/...",
    "template": "Release finished"
  }'
```

- `kind`: `slack`, `mattermost` or `webhook`
- `agentId`: `null` for every agent, or one agent
- `events`: `succeeded`, `failed`, `cancelled` (default: all three)
- `webhookUrl`: write-only; encrypted at rest and never returned

For Slack/Mattermost use an **incoming webhook URL**. The body is `{"text": "..."}`.

For `webhook`, ORBIT POSTs JSON with `event`, `text` and an `execution` object.

## Test

```bash
curl -X POST "$API/api/v1/outputs/$OUTPUT_ID/test" \
  -H "Authorization: Bearer $ORBIT_TOKEN"
```

## Delivery

- A background outbox loop enqueues terminal executions and retries failed deliveries with
  backoff (5 attempts, then `dead`).
- Deliveries are unique per `(output, execution)`.
- List outputs: `GET /api/v1/outputs`; update/delete via `PATCH/DELETE /api/v1/outputs/{id}`.

Not yet implemented: UI, HMAC signing for generic webhooks, SSRF/private-network blocking,
team scoping, delivery log in the UI, and template rendering beyond the built-in summary.

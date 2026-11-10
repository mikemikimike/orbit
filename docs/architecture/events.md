# Events & Triggers

## Transport

``crates/orbit-events`` exposes:

```rust
#[async_trait]
pub trait EventBus: Send + Sync {
    async fn publish(&self, event: &OrbitEvent) -> Result<(), EventBusError>;
    async fn subscribe(&self, filter: SubjectFilter) -> Result<EventStream, EventBusError>;
    async fn health(&self) -> Result<(), EventBusError>;
}
```

Drivers: `memory` (tokio broadcast, development and tests) and `nats` (JetStream,
durable stream `ORBIT`). Selected with `ORBIT_EVENTS_DRIVER`; the API and worker must
agree. JetStream consumers are durable (`orbit-worker`), with explicit ack and a max
delivery count, so worker crashes redeliver.

## Subjects

```
orbit.execution.requested      execution accepted, awaiting dispatch
orbit.execution.dispatch       worker claim / harness assignment
orbit.execution.updated        status transition (also drives UI tail)
orbit.execution.event          individual harness event
orbit.trigger.fired            schedule/webhook/connector event matched a trigger
orbit.connector.event          normalized inbound connector event
orbit.agent.event              agent-emitted event (agent-to-agent, Phase 3)
orbit.worker.heartbeat         harness worker liveness
```

Envelope:

```json
{
  "id": "01J…",
  "type": "orbit.execution.requested",
  "subject": "orbit.execution.requested",
  "source": "orbit-api",
  "time": "2025-01-01T00:00:00Z",
  "org_id": "…",
  "idempotency_key": "exec:9f0c…",
  "payload": { "execution_id": "…", "agent_id": "…" },
  "trace": { "traceparent": "00-…" }
}
```

## Reliability

- At-least-once delivery; consumers are idempotent.
- `processed_events(event_id PRIMARY KEY)` records handled events; duplicate inserts are
  ignored and the message acked.
- Execution claiming is a SQL compare-and-set (`queued → dispatched`), so duplicate
  delivery cannot double-run an execution.
- Failures: `queued` executions older than `ORBIT_EXECUTION_RECLAIM_SECONDS` are
  re-published by the worker reaper; `running` executions with a dead worker heartbeat
  transition to `failed` with a diagnostic event (Phase 1 hardening).
- Terminal states are never overwritten.

## Trigger routing

`TriggerRouter` (worker) consumes `orbit.trigger.fired` and `orbit.connector.event`,
loads enabled triggers for the org, matches them (subject filters, cron occurrences,
webhook ids), and creates executions through the same validated path as the API. The
matcher consults `policies` before enqueueing; denied triggers produce an audit record
and no execution.

## Scheduling (Phase 0 scope)

Cron triggers are stored and validated; the worker includes a lightweight scheduler loop
that computes the next occurrence and publishes `orbit.trigger.fired` (single-worker
safe with a DB advisory lock). Multi-replica HA scheduling is a Phase 2/3 item.

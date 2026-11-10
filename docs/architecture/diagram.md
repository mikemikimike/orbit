# ORBIT architecture diagrams

![ORBIT architecture overview](orbit-overview.svg)

A high-resolution PNG is available at [`orbit-overview.png`](orbit-overview.png).

## Reading the diagram

| Layer | Responsibility |
|---|---|
| **Agent Experience** | Web UI (and future Expo app). Configuration, monitoring, approvals. No business logic or durable state. |
| **Control Plane** | `orbit-api`: auth, agents and immutable versions, providers, connections, executions, approvals, audit, SSE. PostgreSQL is the durable source of truth. NATS JetStream accelerates events; DB polling is the recovery path. |
| **Execution Plane** | `orbit-worker` (mock/scheduler) and `orbit-worker-opencode` (real OpenCode). Workers claim only their configured runtimes. The policy-enforcing tool broker runs per execution. |
| **External systems** | GitHub, X, GitLab, webhooks and model providers. All actions go through the broker; credentials never reach the runtime. |

| Status | Components |
|---|---|
| **Implemented** | API, PostgreSQL schema, NATS/memory bus, mock runtime, OpenCode v2.2.5, X read hydration, SSE, audit, encrypted secrets |
| **In progress** | Execution-scoped tool broker, GitHub capabilities, approval UI |
| **Planned** | GitLab actions, X posting, webhook verification, long-running service mode, swarm simulations, per-execution container isolation |

## System context

```mermaid
flowchart LR
  subgraph Experience["Agent Experience"]
    Web["Web app (Next.js)"]
    Mobile["Future Expo app"]
  end

  subgraph Control["Control Plane"]
    API["orbit-api (Axum)"]
    DB[("PostgreSQL")]
    Bus{{"NATS JetStream"}}
  end

  subgraph Execution["Execution Plane"]
    Worker["orbit-worker<br/>mock · scheduler"]
    OCWorker["orbit-worker-opencode"]
    Runtime["OpenCode CLI<br/>(isolated workspace)"]
    Broker["Policy-enforcing<br/>tool broker"]
  end

  subgraph External["External systems"]
    GitHub["GitHub API"]
    X["X API"]
    More["GitLab · webhooks ·<br/>model providers"]
  end

  Web -->|"REST /api/v1 + SSE"| API
  Mobile -.->|"same API"| API
  API <--> DB
  API <--> Bus
  Bus --> Worker
  Bus --> OCWorker
  Worker <--> DB
  OCWorker <--> DB
  OCWorker -->|"spawn per execution"| Runtime
  Runtime <-->|"MCP tools"| Broker
  Broker -->|"HTTPS + leased credentials"| GitHub
  Broker -->|"HTTPS + leased credentials"| X
  Broker -->|"HTTPS"| More
  API -.->|"approvals / audit"| DB
  Broker -.->|"approval requests, events, audit"| DB
```

## Execution sequence (manual run with a GitHub write)

```mermaid
sequenceDiagram
  autonumber
  actor User
  participant Web
  participant API as orbit-api
  participant DB as PostgreSQL
  participant Bus as NATS
  participant W as orbit-worker-opencode
  participant OC as OpenCode
  participant B as Tool broker
  participant GH as GitHub

  User->>Web: Run agent + task
  Web->>API: POST /agents/{id}/executions
  API->>DB: insert execution (queued, idempotency key)
  API->>Bus: publish execution.requested
  API-->>Web: 202 Accepted
  Web->>API: GET /executions/{id}/events/stream (SSE)
  W->>DB: claim execution (queued → dispatched)
  W->>DB: mark running, append events
  W->>B: start execution-scoped broker (token, allowlist)
  W->>OC: opencode run --standalone --format json
  OC->>B: MCP tools/list
  B-->>OC: allowed GitHub tools only
  OC->>B: tools/call github_issues_comment
  B->>DB: policy = require approval, create approval
  B->>DB: append tool_call event
  Web->>API: GET /approvals (pending)
  User->>Web: Approve
  Web->>API: POST /approvals/{id}/approve
  API->>DB: decision + audit
  B->>DB: poll decision → approved
  B->>GH: POST comment (credential leased here only)
  GH-->>B: 201 Created
  B-->>OC: tool result
  OC-->>W: NDJSON text/tool/usage events
  W->>DB: persist events, finish execution
  W->>Bus: publish execution.updated
  API-->>Web: SSE snapshot + events + done
```

## Tool broker decision path

```mermaid
flowchart TD
  Call["Runtime calls tool via MCP"] --> Token{"Execution token valid?"}
  Token -- no --> Deny401["401 unauthorized + audit"]
  Token -- yes --> Allowed{"Capability in agent +<br/>connection allowlist?"}
  Allowed -- no --> Deny403["403 denied + event + audit"]
  Allowed -- yes --> Effect{"Capability effect"}
  Effect -- read --> Invoke["Lease credential, invoke connector"]
  Effect -- "write / side effect" --> Level{"Autonomy level"}
  Level -- "L4 + allowlisted" --> Invoke
  Level -- "L3 prepare" --> Approval["Create approval, block tool call"]
  Level -- "L1/L2 observe/recommend" --> Deny403
  Approval --> Human{"Human decision"}
  Human -- "rejected / expired" --> Denied["Tool error + audit"]
  Human -- approved --> Invoke
  Invoke --> Result["Tool result → runtime + events + audit"]
```

## Domain model

```mermaid
erDiagram
  ORGANIZATION ||--o{ USER : has
  ORGANIZATION ||--o{ AGENT : owns
  ORGANIZATION ||--o{ PROVIDER : configures
  ORGANIZATION ||--o{ CONNECTION : authorizes
  ORGANIZATION ||--o{ POLICY : enforces

  AGENT ||--o{ AGENT_VERSION : versions
  AGENT ||--o{ AGENT_INSTANCE : deploys
  AGENT ||--o{ TRIGGER : has
  AGENT ||--o{ EXECUTION : runs

  AGENT_VERSION ||--o{ EXECUTION : snapshot
  RUNTIME ||--o{ AGENT_INSTANCE : provides
  CONNECTOR ||--o{ CONNECTION : instantiates

  EXECUTION ||--o{ EXECUTION_EVENT : emits
  EXECUTION ||--o{ APPROVAL : may_require

  ORGANIZATION {
    uuid id
    text slug
    text mode
  }
  AGENT {
    uuid id
    text slug
    text name
    text status
    int current_version
  }
  AGENT_VERSION {
    uuid id
    int version
    jsonb spec
    text checksum
  }
  EXECUTION {
    uuid id
    text status
    text trigger_source
    jsonb input
    jsonb output
    jsonb error
    jsonb usage
  }
  APPROVAL {
    uuid id
    text status
    jsonb requested_action
    uuid decided_by
  }
  CONNECTION {
    uuid id
    text status
    text credential_ref
    text scopes
  }
```

## Deployment topology

```mermaid
flowchart TB
  subgraph Local["Local / Docker Compose"]
    LWeb["web :3000"] --> LApi["api :8080"]
    LWorker["worker (mock)"]
    LOpencode["worker-opencode"]
    LDb[("postgres")]
    LNats{{"nats -js"}}
    LApi <--> LDb
    LApi <--> LNats
    LWorker <--> LDb
    LWorker <--> LNats
    LOpencode <--> LDb
    LOpencode <--> LNats
  end

  subgraph K8s["Kubernetes (Helm)"]
    Ing["Ingress<br/>orbit.mdk.local"] --> SvcWeb["Service: web"]
    Ing -->|"/api, /docs, /healthz"| SvcApi["Service: api"]
    WebD["Deployment: web"] --> SvcWeb
    ApiD["Deployment: api"] --> SvcApi
    WorkerD["Deployment: worker<br/>ORBIT_WORKER_RUNTIMES=mock"]
    OpencodeD["Deployment: worker-opencode<br/>ORBIT_WORKER_RUNTIMES=opencode"]
    Pg[("PostgreSQL<br/>bundled or external")]
    Nats[("NATS<br/>bundled or external")]
    Secret["Secrets: DB URL, master key,<br/>OIDC, OpenCode auth export"]
    ApiD <--> Pg
    ApiD <--> Nats
    WorkerD <--> Pg
    OpencodeD <--> Pg
    OpencodeD <--> Nats
    OpencodeD -.-> Secret
  end
```

## Boundaries that keep the design honest

1. **The control plane never runs agent code.** Only workers and runtime containers do.
2. **PostgreSQL is the source of truth.** The event bus is an accelerator, not a queue you can lose.
3. **Policy and approvals live outside the model.** Prompts are not a security boundary.
4. **Credentials are leased by the broker per action**, never embedded in prompts, CLI arguments or tool output.
5. **Agent versions are immutable.** Running executions keep the version they started with.
6. **Workers are runtime-scoped.** The mock worker cannot claim OpenCode executions and vice versa.
7. **External content is untrusted.** Trusted instructions travel separately from retrieved data.

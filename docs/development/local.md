# Local Development

## Prerequisites
- Docker (Compose v2) and make
- Rust stable (>= 1.85; pinned via `rust-toolchain.toml`)
- Node.js >= 20 and pnpm 10+ (`corepack enable && corepack prepare pnpm@latest --activate`)

## Fast path (containers)

```bash
cp .env.example .env
docker compose -f deploy/compose/compose.yaml up --build
```

- Web UI: http://localhost:3000
- API: http://localhost:8080/api/v1 (docs at http://localhost:8080/docs)
- The web app calls `/api/v1` on its own origin; Next.js proxies to
  `ORBIT_API_INTERNAL_URL` at runtime (default `http://localhost:8080`).
- Postgres: `localhost:5432` (`orbit`/`orbit`, db `orbit`)
- NATS: `localhost:4222`, monitoring `http://localhost:8222`

The API and worker run migrations on startup and seed demo data when
`ORBIT_SEED_DEMO=true`.

## Native dev loop (recommended for coding)

```bash
# infrastructure only
docker compose -f deploy/compose/compose.dev.yaml up -d
cp .env.example .env

cargo run -p orbit-api      # terminal 1
cargo run -p orbit-worker   # terminal 2
pnpm install && pnpm --filter @orbit/web dev   # terminal 3
```

Auth in dev: `ORBIT_AUTH_MODE=dev`, user `dev@orbit.local` with role `owner`.
Set `ORBIT_DEV_AUTH_ALLOW_ANONYMOUS=true` to omit `Authorization` headers.

## Migrations

```bash
cargo install sqlx-cli --no-default-features --features postgres
export DATABASE_URL=postgres://orbit:orbit@localhost:5432/orbit
sqlx migrate add -r <name>          # new migration (in crates/orbit-store/migrations)
sqlx migrate run
sqlx migrate info
```

Migrations are embedded in the binaries and applied automatically at startup
(`ORBIT_DB_AUTO_MIGRATE=true`, default in dev).

## API client generation

```bash
# with the API running
pnpm api:generate
```

Writes `packages/api-client/src/schema.d.ts` from `/api/v1/openapi.json`.

## Useful commands

```bash
cargo fmt --all
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace                     # unit tests
ORBIT_TEST_DATABASE_URL=postgres://… cargo test --workspace -- --include-ignored
make helm-lint
make compose-config
```

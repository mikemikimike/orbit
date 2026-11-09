#!/usr/bin/env bash
# End-to-end smoke test: health, agent creation, manual execution, terminal status and
# persisted events. Works against Compose, a port-forwarded pod, or the ingress.
#
# Usage: API_URL=http://localhost:8080 ./scripts/smoke-k8s.sh
set -euo pipefail

API_URL="${API_URL:-http://localhost:8080}"
TOKEN="${ORBIT_API_TOKEN:-orbit-dev-token}"

api() { curl -sS -H "Authorization: Bearer ${TOKEN}" -H "content-type: application/json" "$@"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

echo "==> Health"
api "${API_URL}/healthz" >/dev/null || fail "health endpoint"
api "${API_URL}/readyz" | grep -q '"status":"ready"' || fail "not ready"

echo "==> Create agent"
AGENT_JSON=$(cat <<'JSON'
{
  "apiVersion": "orbit.io/v1",
  "kind": "Agent",
  "metadata": { "name": "Smoke Test Agent", "description": "created by scripts/smoke-k8s.sh" },
  "spec": {
    "runtime": { "type": "mock", "executionMode": "on_demand", "config": {} },
    "model": { "management": "runtime" },
    "instructions": { "system": "You are a smoke test agent. Summarize the input." },
    "connectors": [],
    "triggers": [],
    "autonomy": { "level": 2 },
    "resources": { "timeoutSeconds": 60, "maxConcurrentExecutions": 1 }
  }
}
JSON
)
AGENT=$(api -X POST "${API_URL}/api/v1/agents" -d "${AGENT_JSON}")
AGENT_ID=$(echo "${AGENT}" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "    agent ${AGENT_ID}"
api -X POST "${API_URL}/api/v1/agents/${AGENT_ID}/activate" >/dev/null

echo "==> Queue execution"
EXECUTION=$(api -X POST "${API_URL}/api/v1/agents/${AGENT_ID}/executions" \
  -H "idempotency-key: smoke-$(date +%s)" \
  -d '{"task":"Reply with a one-line smoke test acknowledgement."}')
EXECUTION_ID=$(echo "${EXECUTION}" | python3 -c 'import json,sys; print(json.load(sys.stdin)["id"])')
echo "    execution ${EXECUTION_ID}"

echo "==> Wait for terminal status"
STATUS=""
for _ in $(seq 1 30); do
  sleep 1
  STATUS=$(api "${API_URL}/api/v1/executions/${EXECUTION_ID}" | python3 -c 'import json,sys; print(json.load(sys.stdin)["status"])')
  case "${STATUS}" in
    succeeded) break ;;
    failed|timed_out|cancelled|rejected) fail "execution ended as ${STATUS}" ;;
  esac
done
[ "${STATUS}" = "succeeded" ] || fail "execution did not finish (last status: ${STATUS})"

echo "==> Verify events"
EVENTS=$(api "${API_URL}/api/v1/executions/${EXECUTION_ID}/events")
COUNT=$(echo "${EVENTS}" | python3 -c 'import json,sys; print(len(json.load(sys.stdin)["items"]))')
[ "${COUNT}" -ge 2 ] || fail "expected recorded events, got ${COUNT}"

echo "PASS: agent -> execution -> ${STATUS} with ${COUNT} events"

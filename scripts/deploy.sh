#!/usr/bin/env bash
# Deploy ORBIT to Kubernetes with Helm.
#
# Usage:
#   KUBE_CONTEXT=microk8s NAMESPACE=orbit VALUES=deploy/helm/orbit/values.yaml ./scripts/deploy.sh
#   TAG=0.1.0 ./scripts/deploy.sh --set ingress.host=orbit.mdk.local
set -euo pipefail

NAMESPACE="${NAMESPACE:-orbit}"
KUBE_CONTEXT="${KUBE_CONTEXT:-microk8s}"
RELEASE="${RELEASE:-orbit}"
VALUES="${VALUES:-deploy/helm/orbit/values.yaml}"
REGISTRY="${REGISTRY:-192.168.1.201:5000}"
TAG="${TAG:-0.1.0}"
INGRESS_HOST="${INGRESS_HOST:-orbit.mdk.local}"

cd "$(dirname "$0")/.."

echo "==> Checking cluster access (${KUBE_CONTEXT})"
if ! kubectl --context "${KUBE_CONTEXT}" get nodes >/dev/null 2>&1; then
  echo "ERROR: cannot reach cluster '${KUBE_CONTEXT}'." >&2
  echo "       Check VPN/LAN access, then retry. No resources were changed." >&2
  exit 1
fi

echo "==> Applying secret prerequisites (namespace ${NAMESPACE})"
kubectl --context "${KUBE_CONTEXT}" create namespace "${NAMESPACE}" \
  --dry-run=client -o yaml | kubectl --context "${KUBE_CONTEXT}" apply -f -

set -x
helm --kube-context "${KUBE_CONTEXT}" upgrade --install "${RELEASE}" deploy/helm/orbit \
  --namespace "${NAMESPACE}" \
  -f "${VALUES}" \
  --set imageRegistry="${REGISTRY}" \
  --set api.tag="${TAG}" \
  --set worker.tag="${TAG}" \
  --set web.tag="${TAG}" \
  --set launcher.tag="${TAG}" \
  --set ingress.host="${INGRESS_HOST}" \
  "$@"
set +x

echo
kubectl --context "${KUBE_CONTEXT}" -n "${NAMESPACE}" get pods,svc,ingress
echo
echo "Ingress: http://${INGRESS_HOST}/  (configure DNS/hosts to the ingress address)"

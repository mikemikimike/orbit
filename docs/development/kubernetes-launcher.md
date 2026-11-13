# Kubernetes worker launch authentication

## Overview

The `orbit-launcher` service is the only component with cluster privileges. It shells
out to `kubectl` to create and manage worker pods. The API never receives kubectl
or Docker socket access — it stores launch intent and calls the launcher over a
separate authenticated HTTP channel.

## How authentication works

### 1. Kubeconfig (default)

The launcher container mounts a kubeconfig file:

```yaml
volumeMounts:
  - name: kubeconfig
    mountPath: /home/orbit/.kube
```

`kubectl` automatically picks up `~/.kube/config` (or the `KUBECONFIG` env var).

### 2. In-cluster service account (recommended for production)

If the launcher runs inside the cluster, it uses the pod's mounted service account
token automatically:

```
/var/run/secrets/kubernetes.io/serviceaccount/token
```

No kubeconfig needed — kubectl detects the in-cluster environment.

## RBAC requirements

The launcher needs these permissions in the target namespace:

```yaml
rules:
- apiGroups: [""]
  resources: ["pods"]
  verbs: ["get", "list", "create", "delete"]
- apiGroups: [""]
  resources: ["secrets"]
  verbs: ["get"]
```

The Helm chart creates a dedicated `ServiceAccount` and `Role`/`RoleBinding` for this.

## Image pull secrets

If your registry requires authentication:

```yaml
ORBIT_LAUNCHER_IMAGE_PULL_SECRET=registry-credentials
```

The launcher adds `imagePullSecrets` to every pod it creates.

## Key environment variables

| Variable | Purpose |
|---|---|
| `ORBIT_LAUNCHER_KUBECTL` | Path to kubectl (default `kubectl`) |
| `ORBIT_LAUNCHER_NAMESPACE` | Target namespace for worker pods (default `orbit`) |
| `ORBIT_LAUNCHER_IMAGE_PULL_SECRET` | Secret name for registry auth |
| `ORBIT_LAUNCHER_IMAGE_ALIASES` | Rewrite registry hosts (e.g. `registry.example.com:5000=registry.example.com:5000`) |
| `ORBIT_LAUNCHER_TOKEN` | Bearer token the API uses to authenticate to the launcher |

## Security model

- The API stores launch intent and calls `POST /launches` on the launcher with a
  bearer token (`ORBIT_LAUNCHER_TOKEN`).
- The launcher validates the token, then runs `kubectl create -f -` with the pod
  manifest.
- Worker pods receive `ORBIT_WORKER_TOKEN` (a scoped worker token) via env var.
- The launcher never exposes kubectl or the Docker socket to the API or web app.
EOF
echo done
#!/usr/bin/env bash
# Fail when the Rust, Node and Helm versions are not aligned.
set -euo pipefail
cd "$(dirname "$0")/.."

python3 - <<'PY'
import json
import pathlib
import re
import sys

versions = {}

cargo = pathlib.Path("Cargo.toml").read_text()
match = re.search(r'(?m)^\[workspace\.package\][\s\S]*?^version\s*=\s*"([^"]+)"', cargo)
versions["Cargo.toml"] = match.group(1) if match else "missing"

for name in [
    "package.json",
    "apps/web/package.json",
    "packages/api-client/package.json",
    "packages/contracts/package.json",
]:
    path = pathlib.Path(name)
    if path.exists():
        versions[name] = json.loads(path.read_text()).get("version", "missing")

chart = pathlib.Path("deploy/helm/orbit/Chart.yaml").read_text()
version = re.search(r"(?m)^version:\s*(\S+)", chart)
app_version = re.search(r'(?m)^appVersion:\s*"?([^"\s]+)"?', chart)
versions["Chart.yaml version"] = version.group(1) if version else "missing"
versions["Chart.yaml appVersion"] = app_version.group(1) if app_version else "missing"

unique = set(versions.values())
for name, value in versions.items():
    print(f"{name}: {value}")
if len(unique) != 1:
    print("ERROR: versions are not aligned", file=sys.stderr)
    sys.exit(1)
PY

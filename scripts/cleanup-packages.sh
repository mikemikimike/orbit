#!/usr/bin/env bash
# Prune old GHCR container versions, Actions artifacts and build-cache artifacts.
# Run with DRY_RUN=true first. Protects the newest releases and their minor tags.
set -euo pipefail

ORG="${ORG:?ORG is required}"
REPO="${REPO:?REPO is required}"
DRY_RUN="${DRY_RUN:-true}"
KEEP_RELEASES="${KEEP_RELEASES:-10}"
MIN_AGE_DAYS="${MIN_AGE_DAYS:-7}"
SHA_AGE_DAYS="${SHA_AGE_DAYS:-30}"
ARTIFACT_AGE_DAYS="${ARTIFACT_AGE_DAYS:-14}"
PACKAGE_PREFIX="${PACKAGE_PREFIX:-orbit}"

PACKAGES=(
  orbit-api
  orbit-worker
  orbit-web
  orbit-launcher
  orbit-worker-opencode
  orbit-worker-pi
)

declare -A PROTECTED
PROTECTED[latest]=1
PROTECTED[main]=1

# Newest N semver releases.
mapfile -t TAGS < <(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname | head -n "$KEEP_RELEASES")
for tag in "${TAGS[@]}"; do
  version="${tag#v}"
  PROTECTED["$version"]=1
  PROTECTED["${version%.*}"]=1
done

# Keep the newest two patches per minor line even beyond KEEP_RELEASES.
declare -A MINOR_COUNT
while read -r tag; do
  version="${tag#v}"
  minor="${version%.*}"
  count="${MINOR_COUNT[$minor]:-0}"
  if (( count < 2 )); then
    PROTECTED["$version"]=1
    PROTECTED["${minor}"]=1
    MINOR_COUNT["$minor"]=$((count + 1))
  fi
done < <(git tag -l 'v[0-9]*.[0-9]*.[0-9]*' --sort=-v:refname)

now=$(date +%s)
age_days() {
  python3 - "$1" <<'PY_AGE'
import datetime, sys
created = sys.argv[1].replace("Z", "+00:00")
dt = datetime.datetime.fromisoformat(created)
print((datetime.datetime.now(datetime.timezone.utc) - dt).days)
PY_AGE
}

echo "Dry run: $DRY_RUN"
echo "Keep releases: $KEEP_RELEASES, min age: ${MIN_AGE_DAYS}d, sha age: ${SHA_AGE_DAYS}d, artifacts: ${ARTIFACT_AGE_DAYS}d"
echo "Protected tags: ${!PROTECTED[*]}"

for package in "${PACKAGES[@]}"; do
  encoded=$(python3 -c 'import urllib.parse,sys; print(urllib.parse.quote(sys.argv[1], safe=""))' "$PACKAGE_PREFIX/$package")
  echo "==> $package"
  while IFS=$'\t' read -r version_id created tags_csv; do
    [[ -z "$version_id" || "$version_id" == "null" ]] && continue
    age=$(age_days "$created")
    IFS=',' read -r -a tags <<< "$tags_csv"

    skip=false
    has_real_tag=false
    has_sha_tag=false
    for tag in "${tags[@]}"; do
      [[ -z "$tag" ]] && continue
      has_real_tag=true
      [[ -n "${PROTECTED[$tag]:-}" ]] && skip=true
      if [[ "$tag" =~ ^sha256- || "$tag" =~ ^[0-9a-f]{40}$ || "$tag" =~ ^sha- ]]; then
        has_sha_tag=true
      fi
    done

    if $skip; then
      echo "  keep $version_id [$tags_csv] protected"
      continue
    fi
    if ! $has_real_tag; then
      echo "  keep $version_id (untagged)"
      continue
    fi
    if (( age < MIN_AGE_DAYS )); then
      echo "  keep $version_id [$tags_csv] recent (${age}d)"
      continue
    fi
    if $has_sha_tag; then
      if (( age < SHA_AGE_DAYS )); then
        echo "  keep $version_id [$tags_csv] sha recent (${age}d)"
        continue
      fi
    fi

    echo "  delete $version_id [$tags_csv] age=${age}d"
    if [[ "$DRY_RUN" != "true" ]]; then
      gh api --method DELETE "orgs/$ORG/packages/container/$encoded/versions/$version_id" >/dev/null
    fi
  done < <(gh api --paginate "orgs/$ORG/packages/container/$encoded/versions?per_page=100" \
    --jq '.[] | [.id, .created_at, ([.metadata.container.tags[]?] | join(","))] | @tsv')
done

echo "==> Actions artifacts"
while IFS=$'\t' read -r artifact_id created; do
  [[ -z "$artifact_id" || "$artifact_id" == "null" ]] && continue
  age=$(age_days "$created")
  if (( age <= ARTIFACT_AGE_DAYS )); then
    continue
  fi
  echo "  delete artifact $artifact_id age=${age}d"
  if [[ "$DRY_RUN" != "true" ]]; then
    gh api --method DELETE "repos/$REPO/actions/artifacts/$artifact_id" >/dev/null
  fi
done < <(gh api --paginate "repos/$REPO/actions/artifacts?per_page=100" \
  --jq '.artifacts[] | [.id, .created_at] | @tsv')

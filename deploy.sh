#!/bin/bash
# Build, push, and deploy mkube-console to mkube on rose1
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REGISTRY="registry.gt.lo:5000"
MKUBE_API="http://192.168.200.2:8082"
IMAGE="$REGISTRY/mkube-console:edge"

echo "=== Deploying mkube-console ==="

# Build
"$SCRIPT_DIR/build.sh"

# Push to local registry
echo "Pushing to $REGISTRY..."
podman push --tls-verify=false "$IMAGE"

# Trigger mkube to pick up the new image immediately
echo "Triggering image redeploy..."
curl -s -X POST "$MKUBE_API/api/v1/images/redeploy" || true

echo ""
echo "=== Done ==="
echo "Deployed mkube-console to $REGISTRY"
echo "Pod: console.gt @ console.gt.lo:8080 (auto-updated by mkube)"

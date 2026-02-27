#!/bin/bash
# Build, push, and deploy mkube-console to mkube on rose1
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REGISTRY="registry.gt.lo:5000"
IMAGE="$REGISTRY/mkube-console:edge"

echo "=== Deploying mkube-console ==="

# Build
"$SCRIPT_DIR/build.sh"

# Push to local registry (mkube will detect and redeploy)
echo "Pushing to $REGISTRY..."
podman push --tls-verify=false "$IMAGE"

echo ""
echo "=== Done ==="
echo "Deployed mkube-console to $REGISTRY"
echo "Pod: console.gt @ console.gt.lo:8080 (auto-updated by mkube)"

#!/bin/bash
set -euo pipefail

echo "==> Building..."
./build.sh

echo "==> Pushing to GHCR..."
podman push ghcr.io/glennswest/mkube-console:edge

echo "==> Done. Registry watcher will mirror to local registry."
echo "    Then deploy via: curl -s -X POST http://192.168.200.2:8082/api/v1/namespaces/gt/pods -H 'Content-Type: application/json' -d @pod.json"

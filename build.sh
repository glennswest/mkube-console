#!/bin/bash
# Build mkube-console: cross-compile locally, then podman build
set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REGISTRY="registry.gt.lo:5000"
IMAGE="$REGISTRY/mkube-console:edge"

echo "=== Building mkube-console ==="

# Cross-compile for ARM64 Linux (static musl)
echo "Building binary for aarch64-unknown-linux-musl..."
cargo build --release --target aarch64-unknown-linux-musl

# Copy binary to project root for Dockerfile
cp target/aarch64-unknown-linux-musl/release/mkube-console mkube-console

# Build scratch container image with podman
echo "Building container image..."
podman build --platform linux/arm64 -t "$IMAGE" .

# Clean up local binary copy
rm -f mkube-console

echo ""
echo "=== Build complete ==="
echo "Image: $IMAGE"
echo "Run ./deploy.sh to push and deploy to rose1"

#!/bin/bash
set -euo pipefail

echo "==> Building mkube-console for aarch64-unknown-linux-musl..."
cargo build --release --target aarch64-unknown-linux-musl

echo "==> Copying binary to build context..."
cp target/aarch64-unknown-linux-musl/release/mkube-console .

echo "==> Building container image..."
podman build --platform linux/arm64 -t ghcr.io/glennswest/mkube-console:edge .

echo "==> Cleaning up..."
rm -f mkube-console

echo "==> Done. Push with: podman push ghcr.io/glennswest/mkube-console:edge"

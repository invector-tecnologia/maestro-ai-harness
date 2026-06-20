#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

pushd "$ROOT_DIR" >/dev/null

echo "[1/5] cargo fmt --all --check"
cargo fmt --all --check

echo "[2/5] cargo check --workspace --all-targets --all-features (rust-analyzer diagnostics)"
cargo check --workspace --all-targets --all-features

echo "[3/5] cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "[4/5] cargo test --workspace -- --nocapture"
cargo test --workspace -- --nocapture

echo "[5/5] docs link integrity"
bash scripts/check-doc-links.sh

popd >/dev/null

echo "Quality gate passed"

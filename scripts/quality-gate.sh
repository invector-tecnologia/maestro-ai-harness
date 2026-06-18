#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "$0")/.." && pwd)"

pushd "$ROOT_DIR" >/dev/null

echo "[1/4] cargo fmt --all --check"
cargo fmt --all --check

echo "[2/4] cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "[3/4] cargo test --workspace -- --nocapture"
cargo test --workspace -- --nocapture

echo "[4/4] docs link integrity"
bash scripts/check-doc-links.sh

popd >/dev/null

echo "Quality gate passed"

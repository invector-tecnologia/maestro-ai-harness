#!/usr/bin/env bash
set -euo pipefail

echo "[1/4] cargo fmt --all --check"
cargo fmt --all --check

echo "[2/4] cargo clippy --workspace --all-targets --all-features -- -D warnings"
cargo clippy --workspace --all-targets --all-features -- -D warnings

echo "[3/4] cargo test --lib -- --nocapture"
cargo test --lib -- --nocapture

echo "[4/4] docs link integrity"
bash scripts/check-doc-links.sh

echo "Onboarding quality gate passed"

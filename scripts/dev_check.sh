#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/openview-dev-check.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

cd "$ROOT_DIR"

need() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "missing required command: $name" >&2
    exit 127
  fi
}

run() {
  echo "+ $*"
  "$@"
}

need cargo
need rustc
need bash

run cargo fmt --all -- --check
run cargo test --workspace

run cargo run -p openview-cli -- --help

echo "+ cargo run -p openview-cli -- manifest hermes.agent > $TMP_DIR/hermes-manifest.json"
cargo run -p openview-cli -- manifest hermes.agent >"$TMP_DIR/hermes-manifest.json"
test -s "$TMP_DIR/hermes-manifest.json"

echo "+ cargo run -p openview-cli -- queues readiness > $TMP_DIR/queues-readiness.json"
cargo run -p openview-cli -- queues readiness >"$TMP_DIR/queues-readiness.json"
test -s "$TMP_DIR/queues-readiness.json"

echo "+ cargo run -p openview-cli -- evidence export-demo > $TMP_DIR/evidence.json"
cargo run -p openview-cli -- evidence export-demo >"$TMP_DIR/evidence.json"
test -s "$TMP_DIR/evidence.json"

run bash scripts/check_public_copy.sh

if [[ "${OPENVIEW_STRICT:-0}" == "1" ]]; then
  run cargo clippy --workspace --all-targets -- -D warnings
fi

if [[ "${OPENVIEW_RUN_LIVE_III:-0}" == "1" ]]; then
  run bash scripts/e2e_iii_runtime.sh
fi

echo "OpenView dev check passed"

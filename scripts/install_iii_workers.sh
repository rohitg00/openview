#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Install the iii workers OpenView expects.

Usage:
  bash scripts/install_iii_workers.sh [--core-only|--agents-only] [--with-oauth] [--dry-run]

Options:
  --core-only    Install only shared OpenView substrate workers.
  --agents-only  Install only AgentView runner wrapper workers.
  --with-oauth   Also install optional OAuth helper workers.
  --dry-run      Print commands without running them.
  -h, --help     Show this help.
USAGE
}

mode="all"
with_oauth=0
dry_run=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --core-only)
      mode="core"
      ;;
    --agents-only)
      mode="agents"
      ;;
    --with-oauth)
      with_oauth=1
      ;;
    --dry-run)
      dry_run=1
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

core_workers=(
  auth-credentials
  models-catalog
  provider-anthropic
  provider-openai
  provider-router
  turn-orchestrator
  session-tree
  session-inbox
  hook-fanout
  policy-denylist
  approval-gate
  llm-budget
  skills
  shell
  subagent
  git-worktree
  iii-database
  iii-queue
  iii-sandbox
  harness
)

agent_workers=(
  codex-agent
  claude-code-agent
  hermes-agent
  openclaw-agent
  opencode-agent
  gemini-cli-agent
  goose-agent
  aider-agent
  openhands-agent
  crush-agent
  qwen-code-agent
  cursor-agent
  amp-agent
)

oauth_workers=(
  oauth-anthropic
  oauth-openai-codex
)

if [[ "$dry_run" != "1" ]] && ! command -v iii >/dev/null 2>&1; then
  echo "iii CLI was not found on PATH." >&2
  echo "Install it first: curl -fsSL https://install.iii.dev/iii/main/install.sh | sh" >&2
  exit 127
fi

run_add() {
  local worker="$1"
  if [[ "$dry_run" == "1" ]]; then
    printf 'iii worker add %s\n' "$worker"
  else
    iii worker add "$worker"
  fi
}

install_group() {
  local worker
  for worker in "$@"; do
    run_add "$worker"
  done
}

case "$mode" in
  all)
    install_group "${core_workers[@]}"
    install_group "${agent_workers[@]}"
    ;;
  core)
    install_group "${core_workers[@]}"
    ;;
  agents)
    install_group "${agent_workers[@]}"
    ;;
esac

if [[ "$with_oauth" == "1" ]]; then
  install_group "${oauth_workers[@]}"
fi

if [[ "$dry_run" == "1" ]]; then
  echo "dry run only; no workers were installed"
else
  echo "OpenView iii worker install complete"
fi

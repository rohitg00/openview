#!/usr/bin/env bash
set -euo pipefail

# Real iii runtime E2E for OpenView's iii adapter surface.
#
# Default mode expects an iii engine + harness worker stack already running on
# the default local ports. Set OPENVIEW_III_MANAGED=1 to start/stop the local
# iii-hq-workers demo stack for the duration of this script. Add
# OPENVIEW_III_E2E_DATABASE=1 to include iii-database transaction/query/execute
# checks against a durable SQLite task-state file.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
III_BIN="${III_BIN:-/Users/rohitsagent/iii/target/debug/iii}"
DEMO_SCRIPT="${III_DEMO_SCRIPT:-/Users/rohitsagent/work/iii-hq-workers/harness/scripts/demo.sh}"
WORKERS_REPO="${III_WORKERS_REPO:-/Users/rohitsagent/work/iii-hq-workers}"
DEMO_DIR="${III_E2E_DEMO_DIR:-/private/tmp/openview-iii-e2e}"
BRIDGE_URL="${III_BRIDGE_URL:-http://127.0.0.1:3111}"
TIMEOUT_MS="${III_TIMEOUT_MS:-30000}"
MANAGED="${OPENVIEW_III_MANAGED:-0}"
DATABASE_E2E="${OPENVIEW_III_E2E_DATABASE:-0}"
RUN_ID="openview-e2e-$(date +%s)-$$"
MESSAGE_TEXT="OpenView live iii e2e message ${RUN_ID}"
DEMO_ENGINE_WS="${III_DEMO_ENGINE_URL:-ws://127.0.0.1:49134}"
DATABASE_BIN="${III_DATABASE_BIN:-$WORKERS_REPO/iii-database/target/release/iii-database}"
DATABASE_CONFIG="$DEMO_DIR/iii-database.yaml"
DATABASE_FILE="$DEMO_DIR/openview-e2e.db"
STARTED_ENGINE=0

export PATH="$(dirname "$III_BIN"):$PATH"

require_bin() {
  local name="$1"
  if ! command -v "$name" >/dev/null 2>&1; then
    echo "missing required command: $name" >&2
    exit 127
  fi
}

require_file() {
  local path="$1"
  local label="$2"
  if [[ ! -e "$path" ]]; then
    echo "missing $label: $path" >&2
    exit 127
  fi
}

trigger() {
  local function_id="$1"
  local payload="${2:-}"
  if [[ -z "$payload" ]]; then
    payload="{}"
  fi
  "$III_BIN" --use-default-config trigger \
    --function-id "$function_id" \
    --payload "$payload" \
    --timeout-ms "$TIMEOUT_MS"
}

pass() {
  echo "PASS: $1"
}

assert_jq() {
  local json="$1"
  shift
  local label="${!#}"
  set -- "${@:1:$(($# - 1))}"
  if ! jq -e "$@" >/dev/null <<<"$json"; then
    echo "FAIL: $label" >&2
    jq . <<<"$json" >&2 || echo "$json" >&2
    exit 1
  fi
  pass "$label"
}

cleanup() {
  if [[ "$MANAGED" == "1" ]]; then
    if [[ -d "$DEMO_DIR/pids" ]]; then
      for pidfile in "$DEMO_DIR/pids"/*.pid; do
        [[ -f "$pidfile" ]] || continue
        pid="$(cat "$pidfile")"
        if kill -0 "$pid" >/dev/null 2>&1; then
          kill -TERM "$pid" >/dev/null 2>&1 || true
        fi
      done
    fi
    if [[ "$STARTED_ENGINE" == "1" && -f "$DEMO_DIR/engine.pid" ]]; then
      pid="$(cat "$DEMO_DIR/engine.pid")"
      if kill -0 "$pid" >/dev/null 2>&1; then
        kill -TERM "$pid" >/dev/null 2>&1 || true
      fi
    fi
  fi
}

wait_for_harness() {
  local out
  for _ in $(seq 1 40); do
    if out="$(trigger "harness::status" "{}" 2>/dev/null)"; then
      if jq -e '.ok == true' >/dev/null <<<"$out"; then
        pass "harness::status is healthy"
        return 0
      fi
    fi
    sleep 1
  done
  echo "FAIL: harness::status did not become healthy" >&2
  return 1
}

engine_ready() {
  trigger "engine::queue::list_topics" "{}" >/dev/null 2>&1
}

bin_name_for() {
  local worker="$1"
  local yaml="$WORKERS_REPO/$worker/iii.worker.yaml"
  local name
  if [[ -f "$yaml" ]]; then
    name="$(awk '/^bin:/{sub(/^bin:[ \t]*/, ""); sub(/[ \t]+$/, ""); print; exit}' "$yaml")"
    if [[ -n "$name" ]]; then
      echo "$name"
      return
    fi
  fi
  echo "$worker"
}

start_engine() {
  mkdir -p "$DEMO_DIR"
  if engine_ready; then
    return 0
  fi

  "$III_BIN" --use-default-config >"$DEMO_DIR/engine.log" 2>&1 &
  echo $! >"$DEMO_DIR/engine.pid"
  STARTED_ENGINE=1

  for _ in $(seq 1 20); do
    if engine_ready; then
      pass "iii engine is reachable"
      return 0
    fi
    sleep 1
  done

  echo "FAIL: iii engine did not become reachable" >&2
  tail -n 80 "$DEMO_DIR/engine.log" >&2 || true
  exit 1
}

spawn_worker() {
  local worker="$1"
  local bin="$WORKERS_REPO/$worker/target/release/$(bin_name_for "$worker")"
  local pidfile="$DEMO_DIR/pids/$worker.pid"
  local logfile="$DEMO_DIR/logs/$worker.log"
  local -a run_args=()
  local -a extra_env=()

  require_file "$bin" "$worker binary"
  : >"$logfile"

  if [[ "$worker" == "harness" ]]; then
    run_args=(--config "$WORKERS_REPO/harness/config.yaml" --url "$DEMO_ENGINE_WS")
  elif [[ "$worker" == "shell" ]]; then
    run_args=(--config "$WORKERS_REPO/harness/shell-config.yaml")
  fi

  if [[ "$worker" == "policy-denylist" ]]; then
    extra_env+=(POLICY_DENIED_FUNCTIONS="bridge::trigger")
  fi

  if (( ${#extra_env[@]} > 0 && ${#run_args[@]} > 0 )); then
    env III_URL="$DEMO_ENGINE_WS" "${extra_env[@]}" "$bin" "${run_args[@]}" >>"$logfile" 2>&1 &
  elif (( ${#extra_env[@]} > 0 )); then
    env III_URL="$DEMO_ENGINE_WS" "${extra_env[@]}" "$bin" >>"$logfile" 2>&1 &
  elif (( ${#run_args[@]} > 0 )); then
    env III_URL="$DEMO_ENGINE_WS" "$bin" "${run_args[@]}" >>"$logfile" 2>&1 &
  else
    env III_URL="$DEMO_ENGINE_WS" "$bin" >>"$logfile" 2>&1 &
  fi
  echo $! >"$pidfile"
}

start_workers() {
  mkdir -p "$DEMO_DIR/pids" "$DEMO_DIR/logs"
  local workers=(
    turn-orchestrator provider-router
    session-tree session-inbox
    models-catalog hook-fanout policy-denylist
    shell subagent
    provider-anthropic provider-openai
    auth-credentials llm-budget
    skills approval-gate
  )

  for worker in "${workers[@]}"; do
    spawn_worker "$worker"
    sleep 0.1
  done
  spawn_worker harness
}

start_database_worker() {
  mkdir -p "$DEMO_DIR/pids" "$DEMO_DIR/logs"
  require_file "$DATABASE_BIN" "iii-database binary"
  cat >"$DATABASE_CONFIG" <<EOF
databases:
  primary:
    url: sqlite:$DATABASE_FILE
    pool:
      max: 5
      idle_timeout_ms: 30000
      acquire_timeout_ms: 5000
EOF
  : >"$DEMO_DIR/logs/iii-database.log"
  "$DATABASE_BIN" --config "$DATABASE_CONFIG" --url "$DEMO_ENGINE_WS" >>"$DEMO_DIR/logs/iii-database.log" 2>&1 &
  echo $! >"$DEMO_DIR/pids/iii-database.pid"
}

wait_for_database() {
  local out
  for _ in $(seq 1 40); do
    if out="$(trigger "iii-database::query" '{"db":"primary","sql":"SELECT 1 AS ok"}' 2>/dev/null)"; then
      if jq -e '.rows[0].ok == 1' >/dev/null <<<"$out"; then
        pass "iii-database worker is reachable"
        return 0
      fi
    fi
    sleep 1
  done

  echo "FAIL: iii-database worker did not become reachable" >&2
  tail -n 120 "$DEMO_DIR/logs/iii-database.log" >&2 || true
  exit 1
}

bootstrap_queue_schema() {
  local tx_payload tx_json

  tx_payload="$(jq -nc '{
    db:"primary",
    statements:[
      {sql:"CREATE TABLE IF NOT EXISTS durable_tasks (task_id TEXT PRIMARY KEY, queue TEXT NOT NULL, task_name TEXT NOT NULL, payload_ref TEXT NOT NULL, payload_sha256 TEXT NOT NULL, state TEXT NOT NULL CHECK (state IN ('\''queued'\'', '\''leased'\'', '\''retrying'\'', '\''completed'\'', '\''dead_lettered'\'', '\''cancelled'\'')), attempt INTEGER NOT NULL DEFAULT 0, visible_at TEXT NOT NULL, created_at TEXT NOT NULL, updated_at TEXT NOT NULL, lease_id TEXT, lease_owner TEXT, leased_at TEXT, lease_expires_at TEXT, last_heartbeat_at TEXT, output_ref TEXT, output_sha256 TEXT, completed_at TEXT, failure_ref TEXT, failure_kind TEXT, dead_lettered_at TEXT, recovered_by TEXT, recovered_at TEXT)", params:[]},
      {sql:"CREATE TABLE IF NOT EXISTS durable_task_events (event_id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, kind TEXT NOT NULL, payload TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (task_id) REFERENCES durable_tasks(task_id) ON DELETE CASCADE)", params:[]},
      {sql:"CREATE TABLE IF NOT EXISTS durable_dead_letters (task_id TEXT PRIMARY KEY, failure_ref TEXT NOT NULL, failure_kind TEXT NOT NULL, worker_id TEXT NOT NULL, created_at TEXT NOT NULL, FOREIGN KEY (task_id) REFERENCES durable_tasks(task_id) ON DELETE CASCADE)", params:[]},
      {sql:"CREATE TABLE IF NOT EXISTS durable_idempotency_keys (idempotency_key TEXT PRIMARY KEY, task_id TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT (datetime('\''now'\'')))", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_tasks_claim_ready ON durable_tasks (queue, state, visible_at, created_at) WHERE state IN ('\''queued'\'', '\''retrying'\'')", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_tasks_recover_leases ON durable_tasks (queue, state, lease_expires_at, created_at) WHERE state = '\''leased'\''", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_tasks_recover_retries ON durable_tasks (queue, state, visible_at, created_at) WHERE state = '\''retrying'\''", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_tasks_lease_owner ON durable_tasks (lease_id, lease_owner)", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_task_events_task_created ON durable_task_events (task_id, created_at)", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_dead_letters_created ON durable_dead_letters (created_at)", params:[]},
      {sql:"CREATE INDEX IF NOT EXISTS idx_durable_idempotency_task_id ON durable_idempotency_keys (task_id)", params:[]}
    ]
  }')"
  tx_json="$(trigger "iii-database::transaction" "$tx_payload")"
  assert_jq "$tx_json" '.committed == true and (.results | length == 11)' \
    "iii-database transaction applies OpenView queue schema bootstrap"
}

verify_queue_schema() {
  local tables_json indexes_json columns_json

  tables_json="$(trigger "iii-database::query" "$(jq -nc \
    '{db:"primary", sql:"SELECT name FROM sqlite_master WHERE type = '\''table'\'' AND name IN ('\''durable_tasks'\'','\''durable_task_events'\'','\''durable_dead_letters'\'','\''durable_idempotency_keys'\'')"}')")"
  assert_jq "$tables_json" --argjson actual "$tables_json" \
    '([ "durable_tasks", "durable_task_events", "durable_dead_letters", "durable_idempotency_keys" ] | all(. as $name | $actual.rows | any(.name == $name)))' \
    "OpenView durable queue tables exist after bootstrap"

  indexes_json="$(trigger "iii-database::query" "$(jq -nc \
    '{db:"primary", sql:"SELECT name FROM sqlite_master WHERE type = '\''index'\'' AND name IN ('\''idx_durable_tasks_claim_ready'\'','\''idx_durable_tasks_recover_leases'\'','\''idx_durable_tasks_recover_retries'\'','\''idx_durable_tasks_lease_owner'\'','\''idx_durable_task_events_task_created'\'','\''idx_durable_dead_letters_created'\'','\''idx_durable_idempotency_task_id'\'')"}')")"
  assert_jq "$indexes_json" --argjson actual "$indexes_json" \
    '([ "idx_durable_tasks_claim_ready", "idx_durable_tasks_recover_leases", "idx_durable_tasks_recover_retries", "idx_durable_tasks_lease_owner", "idx_durable_task_events_task_created", "idx_durable_dead_letters_created", "idx_durable_idempotency_task_id" ] | all(. as $name | $actual.rows | any(.name == $name)))' \
    "OpenView durable queue indexes exist after bootstrap"

  columns_json="$(trigger "iii-database::query" '{"db":"primary","sql":"PRAGMA table_info(durable_tasks)"}')"
  assert_jq "$columns_json" --argjson actual "$columns_json" \
    '([ "task_id", "queue", "task_name", "payload_ref", "payload_sha256", "state", "attempt", "visible_at", "created_at", "updated_at", "lease_id", "lease_owner", "lease_expires_at", "recovered_by" ] | all(. as $name | $actual.rows | any(.name == $name)))' \
    "OpenView durable queue task columns exist after bootstrap"
}

exercise_database_task_state() {
  local tx_payload tx_json query_json heartbeat_payload heartbeat_json recovery_payload
  local recovery_json claim_payload claim_json retry_payload retry_json dead_letter_payload
  local dead_letter_json replay_json

  tx_payload="$(jq -nc \
    --arg run_id "$RUN_ID" \
    --arg lease_id "lease-initial" \
    --arg owner "openview-e2e-worker" \
    --arg payload_ref "memory://openview-e2e/$RUN_ID/payload" \
    --arg failure_ref "memory://openview-e2e/$RUN_ID/failure" \
    '{
      db:"primary",
      statements:[
        {sql:"INSERT OR REPLACE INTO durable_tasks (task_id,queue,task_name,payload_ref,payload_sha256,state,attempt,visible_at,created_at,updated_at,lease_id,lease_owner,leased_at,lease_expires_at,failure_ref) VALUES (?,?,?,?,?,?,?,?,?,?,?,?,?,?,?)", params:[$run_id,"shell.sandbox","openview.live_iii_e2e",$payload_ref,"sha256:e2e","leased",0,"2026-05-15T10:00:00Z","2026-05-15T10:00:00Z","2026-05-15T10:00:00Z",$lease_id,$owner,"2026-05-15T10:00:00Z","2026-05-15T10:00:30Z",$failure_ref]},
        {sql:"INSERT INTO durable_task_events (task_id,kind,payload,created_at) VALUES (?,?,?,?)", params:[$run_id,"leased","{\"source\":\"live-iii-e2e\"}","2026-05-15T10:00:00Z"]}
      ]
    }')"
  tx_json="$(trigger "iii-database::transaction" "$tx_payload")"
  assert_jq "$tx_json" '.committed == true and (.results | length == 2)' \
    "iii-database transaction persists task lease state"

  query_json="$(trigger "iii-database::query" "$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"SELECT task_id,state,lease_id,lease_owner,attempt,payload_ref FROM durable_tasks WHERE task_id = ?", params:[$run_id]}')")"
  assert_jq "$query_json" --arg run_id "$RUN_ID" \
    '.row_count == 1 and .rows[0].task_id == $run_id and .rows[0].state == "leased" and .rows[0].lease_id == "lease-initial" and .rows[0].lease_owner == "openview-e2e-worker" and (.rows[0].attempt | tonumber) == 0 and (.rows[0].payload_ref | contains($run_id))' \
    "iii-database query reads durable task lease state"

  heartbeat_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET lease_expires_at = ?, last_heartbeat_at = ?, updated_at = ? WHERE task_id = ? AND lease_id = ? AND state = ? AND lease_expires_at > ?", params:["2026-05-15T10:01:00Z","2026-05-15T10:00:20Z","2026-05-15T10:00:20Z",$run_id,"lease-initial","leased","2026-05-15T10:00:20Z"]}')"
  heartbeat_json="$(trigger "iii-database::execute" "$heartbeat_payload")"
  assert_jq "$heartbeat_json" '.affected_rows == 1' \
    "iii-database heartbeat extends a current lease"

  recovery_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET state = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, recovered_by = ?, recovered_at = ?, updated_at = ? WHERE task_id = ? AND state = ? AND lease_expires_at <= ?", params:["queued","openview-recovery","2026-05-15T10:01:01Z","2026-05-15T10:01:01Z",$run_id,"leased","2026-05-15T10:01:01Z"]}')"
  recovery_json="$(trigger "iii-database::execute" "$recovery_payload")"
  assert_jq "$recovery_json" '.affected_rows == 1' \
    "iii-database recovery releases an expired lease"

  claim_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET state = ?, lease_id = ?, lease_owner = ?, leased_at = ?, lease_expires_at = ?, updated_at = ? WHERE task_id = ? AND queue = ? AND state = ? AND visible_at <= ?", params:["leased","lease-recovered","openview-recovery-worker","2026-05-15T10:01:02Z","2026-05-15T10:02:00Z","2026-05-15T10:01:02Z",$run_id,"shell.sandbox","queued","2026-05-15T10:01:02Z"]}')"
  claim_json="$(trigger "iii-database::execute" "$claim_payload")"
  assert_jq "$claim_json" '.affected_rows == 1' \
    "iii-database claim reacquires recovered visible work"

  retry_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET state = CASE WHEN attempt + 1 < ? THEN ? ELSE ? END, attempt = attempt + 1, visible_at = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, failure_kind = ?, updated_at = ? WHERE task_id = ? AND lease_id = ? AND state = ?", params:[2,"retrying","dead_lettered","2026-05-15T10:03:00Z","transient","2026-05-15T10:02:01Z",$run_id,"lease-recovered","leased"]}')"
  retry_json="$(trigger "iii-database::execute" "$retry_payload")"
  assert_jq "$retry_json" '.affected_rows == 1' \
    "iii-database retry reschedules below max attempts"

  claim_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET state = ?, lease_id = ?, lease_owner = ?, leased_at = ?, lease_expires_at = ?, updated_at = ? WHERE task_id = ? AND queue = ? AND state = ? AND visible_at <= ?", params:["leased","lease-final","openview-retry-worker","2026-05-15T10:03:01Z","2026-05-15T10:04:00Z","2026-05-15T10:03:01Z",$run_id,"shell.sandbox","retrying","2026-05-15T10:03:01Z"]}')"
  claim_json="$(trigger "iii-database::execute" "$claim_payload")"
  assert_jq "$claim_json" '.affected_rows == 1' \
    "iii-database claim reacquires retry-ready work"

  dead_letter_payload="$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"UPDATE durable_tasks SET state = CASE WHEN attempt + 1 < ? THEN ? ELSE ? END, attempt = attempt + 1, visible_at = ?, lease_id = NULL, lease_owner = NULL, lease_expires_at = NULL, failure_kind = ?, dead_lettered_at = ?, updated_at = ? WHERE task_id = ? AND lease_id = ? AND state = ?", params:[2,"retrying","dead_lettered","2026-05-15T10:05:00Z","permanent","2026-05-15T10:05:00Z","2026-05-15T10:05:00Z",$run_id,"lease-final","leased"]}')"
  dead_letter_json="$(trigger "iii-database::execute" "$dead_letter_payload")"
  assert_jq "$dead_letter_json" '.affected_rows == 1' \
    "iii-database dead-letters after retry exhaustion"

  replay_json="$(trigger "iii-database::query" "$(jq -nc --arg run_id "$RUN_ID" \
    '{db:"primary", sql:"SELECT state,attempt,lease_id,lease_owner,failure_kind,payload_ref FROM durable_tasks WHERE task_id = ?", params:[$run_id]}')")"
  assert_jq "$replay_json" --arg run_id "$RUN_ID" \
    '.row_count == 1 and .rows[0].state == "dead_lettered" and (.rows[0].attempt | tonumber) == 2 and .rows[0].lease_id == null and .rows[0].lease_owner == null and .rows[0].failure_kind == "permanent" and (.rows[0].payload_ref | contains($run_id))' \
    "iii-database replay read observes dead-lettered durable task state"
}

require_bin jq
require_bin curl
require_file "$III_BIN" "iii CLI"

if [[ "$MANAGED" == "1" ]]; then
  require_file "$WORKERS_REPO" "iii workers repo"
  trap cleanup EXIT
  start_engine
  start_workers
  if [[ "$DATABASE_E2E" == "1" ]]; then
    start_database_worker
  fi
fi

wait_for_harness

if [[ "$DATABASE_E2E" == "1" ]]; then
  wait_for_database
  bootstrap_queue_schema
  verify_queue_schema
  exercise_database_task_state
fi

models_json="$(trigger "models::list" "{}")"
assert_jq "$models_json" '.models | any(.id == "gpt-5" and .supports_xhigh == true)' \
  "model catalog exposes gpt-5 with xhigh support"

stream_payload="$(jq -nc \
  --arg run_id "$RUN_ID" \
  '{stream_name:"openview-e2e", group_id:$run_id, item_id:"plan", data:{goal:"real-e2e", status:"running", run_id:$run_id}}')"
trigger "stream::set" "$stream_payload" >/dev/null
stream_json="$(trigger "stream::list" "$(jq -nc --arg run_id "$RUN_ID" '{stream_name:"openview-e2e", group_id:$run_id}')")"
assert_jq "$stream_json" --arg run_id "$RUN_ID" \
  'any(.run_id == $run_id and .status == "running")' \
  "stream::set/list round-trips a run event"

session_json="$(trigger "session-tree::create" "$(jq -nc --arg run_id "$RUN_ID" --arg cwd "$ROOT_DIR" '{display_name:$run_id, cwd:$cwd}')")"
session_id="$(jq -r '.session_id' <<<"$session_json")"
if [[ -z "$session_id" || "$session_id" == "null" ]]; then
  echo "FAIL: session-tree::create did not return session_id" >&2
  jq . <<<"$session_json" >&2
  exit 1
fi
append_payload="$(jq -nc \
  --arg session_id "$session_id" \
  --arg text "$MESSAGE_TEXT" \
  --argjson ts "$(date +%s)" \
  '{session_id:$session_id, message:{role:"user", content:[{type:"text", text:$text}], timestamp:$ts}}')"
append_json="$(trigger "session-tree::append" "$append_payload")"
entry_id="$(jq -r '.entry_id' <<<"$append_json")"
messages_json="$(trigger "session-tree::messages" "$(jq -nc --arg session_id "$session_id" '{session_id:$session_id}')")"
assert_jq "$messages_json" --arg entry_id "$entry_id" --arg text "$MESSAGE_TEXT" \
  '.messages | any(.entry_id == $entry_id and (.message.content | any(.text == $text)))' \
  "session-tree create/append/messages round-trips transcript state"

approval_json="$(trigger "approval::list_pending" "{}")"
assert_jq "$approval_json" '.pending | type == "array"' \
  "approval::list_pending returns a durable queue shape"

durable_payload="$(jq -nc --arg run_id "$RUN_ID" '{topic:"openview.e2e.topic", data:{run_id:$run_id, status:"queued"}}')"
trigger "iii::durable::publish" "$durable_payload" >/dev/null
topics_json="$(trigger "engine::queue::list_topics" "{}")"
assert_jq "$topics_json" 'map(.name) | index("turn::step_requested") != null' \
  "iii durable queue worker is reachable"

hook_payload="$(jq -nc --arg run_id "$RUN_ID" \
  '{topic:"openview.e2e.hook", payload:{run_id:$run_id, status:"proposed"}, merge_rule:"field_merge", timeout_ms:100}')"
hook_json="$(trigger "hook-fanout::publish_collect" "$hook_payload")"
assert_jq "$hook_json" --arg run_id "$RUN_ID" \
  '.merged.run_id == $run_id and (.replies | type == "array")' \
  "hook-fanout publish/collect executes through iii durable publish and stream reads"

bridge_json="$(curl -fsS --max-time 10 \
  -X POST "$BRIDGE_URL/bridge/trigger" \
  -H 'content-type: application/json' \
  -d '{"function_id":"models::list","payload":{}}')"
assert_jq "$bridge_json" '.models | any(.id == "gpt-5")' \
  "REST bridge invokes iii functions"

shell_payload='{"command":"echo","args":["openview-live-e2e"],"target":{"kind":"host"},"timeout_ms":5000}'
shell_json="$(trigger "shell::exec" "$shell_payload")"
assert_jq "$shell_json" '.exit_code == 0 and .stdout == "openview-live-e2e\n" and .timed_out == false' \
  "shell::exec runs an allowlisted command"

echo "PASS: OpenView live iii runtime E2E completed (${RUN_ID})"

# TASK 044: Interview Live Thinking Feedback — Maestro "Stopped Answering" Fix

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** In the LLM-driven onboarding interview (Option B), the live `Maestro`
  agent answers through the `AgentRuntime` subscriber loop
  (`run_agent_loop` → `process_message_cycle`), whose `think()` awaits
  `LlmProvider::text_only`. While a model call is in flight the Maestro interview
  panel always renders a static `🧠 Thinking with Maestro...` whenever a model is
  online — independent of the real agent state — so a slow or stalled call looks like
  Maestro "stopped answering" with no feedback.
* **Context Anchors:** #file:src/presentation/tui/render.rs,
  #file:src/presentation/tui/app.rs, #file:src/presentation/tui/mod.rs,
  #file:src/application/agent_runtime.rs,
  #file:src/application/persona_operations.rs
* **Expected Output:** The interview panel reflects the real `Maestro` agent health
  with a live elapsed indicator while thinking, a slow-model hint after a threshold,
  an explicit error state, and a listening state when idle — so the user always knows
  whether Maestro is working, slow, errored, or waiting on them.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Reuse the existing `AgentHealth` snapshot already polled by the TUI loop; do not add
  a parallel state channel. The panel must be driven by real health, not a static flag.
* No change to provider timeout semantics: do not add a hard global timeout that could
  cut off legitimately slow local models. Genuine failures continue to surface via the
  existing reqwest timeout → `ExecutionError`/`publish_runtime_error` path.
* No production `unwrap()`/`expect()`/`panic!()` in changed paths; `tracing` for
  observability; presentation logic stays in `presentation`.
* Edition-2018 positional format arguments only (no inline `{var}` captures) where the
  surrounding file already follows that rule.

## 3. ACCEPTANCE CRITERIA
* AC1: The interview Maestro panel renders `🧠 Thinking with Maestro… (Ns)` with a
  live elapsed counter only while the real `Maestro` agent is in the `think` state.
* AC2: When a single `think` exceeds `SLOW_THINK_HINT_SECS` (20s), the panel appends a
  slow-model hint pointing to provider/model and `timeout_ms` in `maestro/config.yml`.
* AC3: When the `Maestro` agent is in the `error` state, the panel surfaces an error
  line directing the user to the Orchestration log; when idle, it shows a listening
  line; approval prompts take precedence over the thinking line.
* AC4: `TuiApp` tracks the thinking start instant, setting it when Maestro enters
  `think` and clearing it as soon as Maestro leaves `think`.
* AC5: All quality gates pass with new tests covering the status-line builder
  (thinking/slow/idle/error/approval) and the thinking-window tracking transitions.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP (PR sequence under this task)
* PR1: Panel reflects real Maestro health — `thinking_since` tracking in `TuiApp`
  (`update_agents_from_health`), a `maestro_status_lines` builder in `render.rs`
  (thinking + elapsed + slow hint + error + listening), and the panel wired to live
  agent state. Tests for the builder and the tracking transitions. (AC1–AC5)

## 6. RESIDUAL RISKS / NOTES
* Subscriber-path runtime events (`process_message_cycle`) are emitted via
  `event_tx.send` and are NOT persisted to `event_history`, so they never reach the
  TUI's `events_snapshot()`. A heartbeat narration in the subscriber loop would
  therefore not surface without additionally threading `event_history` through the
  subscriber path AND resolving the interview log refresh (`update_logs_from_history`
  overwrites `self.logs` each tick, clobbering appended runtime-event lines). The panel
  indicator is intentionally health-driven to be reliable regardless of the log path;
  the heartbeat/log-merge work is deferred as a separate increment.
* Slow generations remain slow: this task improves feedback and guidance, not model
  latency. Users tune `timeout_ms`/model per the surfaced hint.

## 7. VALIDATION EVIDENCE
### PR1 — Interview live thinking feedback (AC1–AC5)
* `TuiApp` gained `thinking_since: Option<Instant>`
  (`src/presentation/tui/mod.rs`); `update_agents_from_health`
  (`src/presentation/tui/app.rs`) sets it when the `Maestro` agent enters `think` and
  clears it otherwise.
* `render_maestro_panel` (`src/presentation/tui/render.rs`) now builds its status
  line(s) via `maestro_status_lines(approval, maestro_status, online, thinking_secs)`,
  driven by the live `Maestro` agent status from `app.agents` and the elapsed time
  from `thinking_since`: live `Thinking… (Ns)`, a slow-model hint past
  `SLOW_THINK_HINT_SECS`, an error line, and a listening line.
* `cargo fmt --all` — clean.
* `cargo clippy --all-targets -- -D warnings` — no warnings.
* `cargo test --all-targets` — 188 passed; 0 failed. New tests:
  `maestro_status_tests::{shows_live_elapsed_while_thinking, appends_slow_hint_past_threshold, listens_when_idle, surfaces_error_state, approval_takes_precedence_over_thinking}`
  and `thinking_since_tracks_maestro_think_state`.
* `scripts/check-doc-links.sh` — link integrity passed.

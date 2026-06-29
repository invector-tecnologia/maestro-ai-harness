# TASK 045: Active Maestro Interview — Guide → Process → Confirm → Write → Hand off

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** In the LLM-driven onboarding interview (Option B) the live `Maestro`
  agent already answers through the `AgentRuntime` subscriber loop and is instructed by
  `maestro_capability_preamble()` to emit a fenced ```json `{"changes":[…]}` proposal
  when ready. However, nothing in the TUI loop reads that proposal: the interview is
  actually driven by the deterministic `InterviewBot` script (fixed questions, heuristic
  analysis at turn 7) and the only files written come from the scripted scope path. The
  live Maestro voice is conversational but never authors anything.
* **Context Anchors:** #file:src/application/interview_bot.rs,
  #file:src/application/persona_operations.rs,
  #file:src/presentation/tui/interview.rs, #file:src/presentation/tui/mod.rs,
  #file:src/presentation/tui/render.rs, #file:src/presentation/cli/mod.rs
* **Expected Output:** When a model is online, Maestro actively drives the interview:
  it asks adaptive questions, and when ready emits a governed-file proposal that the
  harness parses, presents for confirmation, applies through `MarkdownGovernance`, and
  then hands the project off to the Workspace runtime so the full agent team can begin
  building. The deterministic scripted interview remains the offline fallback unchanged.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Reuse the existing, tested building blocks: `parse_directive_proposals`,
  `apply_directive_change`, `maestro_capability_preamble`, and
  `registrations_from_governance`. Do not duplicate parsing or governance logic.
* Every write goes through `apply_directive_change` → `MarkdownGovernance` validation.
  The Maestro persona stays immutable: filter out any change targeting Maestro before
  staging (defense in depth on top of the governance guard).
* Approval is mandatory: no proposed change is persisted without explicit user
  confirmation.
* The LLM-driven path is additive and engine-gated (`InterviewEngine::is_llm_driven`).
  The `GuidedSetup` (offline) heuristic path is preserved byte-for-byte in behavior.
* This supersedes TASK 035's "no LLM-driven synthesis" rule *only* for the parallel
  LLM engine, and only because authored content still passes governance validation;
  the deterministic directive path that 035 governs is unchanged.
* No production `unwrap()`/`expect()`/`panic!()` in changed paths; `thiserror` in
  domain/application, `anyhow` only at the CLI/TUI boundary; `tracing` for observability.
* `Arc<tokio::sync::RwLock<T>>` for shared async interview state. Edition-2018 positional
  format arguments only where the surrounding file already follows that rule.

## 3. ACCEPTANCE CRITERIA
* AC1: When the interview engine is LLM-driven, the harness scans the latest unscanned
  `Maestro` bus message for a fenced/embedded JSON proposal via
  `parse_directive_proposals`, ignoring plain conversational messages and never
  re-parsing a message already scanned (tracked by `last_parsed_maestro_msg`).
* AC2: Parsed changes that target the Maestro persona are dropped before staging; the
  remaining changes are staged in `InterviewSession::pending_changes` with
  `confirmation_pending = true`.
* AC3: A confirmation view lists each staged change (operation + kind + file) and the
  user can approve or refine. Refusing clears the staged changes and re-opens the
  dialogue by posting a refinement message back to Maestro; nothing is written.
* AC4: Approving applies every staged change through `apply_directive_change`
  (governance-validated), logging each written/archived/read path, and counts writes.
* AC5: After a successful apply the harness hands off to the Workspace runtime — it
  rebuilds the governed sequential pipeline (`registrations_from_governance`) on the
  shared runtime and switches the UI to Workspace mode so the user's next instruction
  orchestrates the full agent team.
* AC6: When the engine is LLM-driven, the scripted question/heuristic-analysis path is
  bypassed (Maestro drives); when offline (`GuidedSetup`) the existing scripted path is
  unchanged.
* AC7: All quality gates pass with new unit tests covering proposal detection
  (fenced/prose/already-scanned/Maestro-target filtering), governance application, and
  the Workspace handoff transition.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP (PR sequence under this task)
* PR1: Staging state — add `pending_changes`, `last_parsed_maestro_msg`,
  `confirmation_pending` to `InterviewSession`; `detect_and_stage_maestro_proposal`
  helper in `interview.rs`. (AC1, AC2)
* PR2: Active drive — bypass the scripted question/heuristic path when LLM-driven so
  Maestro's own bus messages are the questions; run detection each loop tick. (AC6)
* PR3: Confirmation — render staged changes in the approval modal; approve/refine
  branch in the loop. (AC3)
* PR4: Write — `apply_directive_changes` through governance with per-change logging. (AC4)
* PR5: Handoff — `handoff_to_workspace` rebuilds the governed sequential pipeline and
  switches to Workspace mode; thread the `ModelRouter` into `run_tui`. (AC5)
* PR6: Docs + evidence — update User Manual / onboarding docs; record validation. (AC7)

## 6. RESIDUAL RISKS / NOTES
* Interview narration uses the environment bus (`env.get_history`), which the TUI
  already renders each tick, so Maestro's questions and confirmations are visible
  without touching the broadcast-only subscriber event path noted in TASK 044.
* A model may emit malformed or oversized JSON: `parse_directive_proposals` is tolerant
  and returns typed errors; on parse failure the message is treated as a normal
  question (no crash), and its id is recorded to avoid reparse loops.
* The handoff stops the interview's single Maestro subscriber and installs the full
  governed pipeline; if no `ModelRouter` is available (provider setup failed), the
  handoff still switches to Workspace mode and logs guidance instead of a pipeline.

## 7. VALIDATION EVIDENCE
* `cargo fmt --all` — clean (no diff).
* `cargo clippy --all-targets -- -D warnings` — passed with no warnings.
* `cargo test --all-targets` — **194 passed; 0 failed** (188 baseline + 6 new Task 045 tests:
  `detect_stages_governed_proposal_from_maestro`, `detect_ignores_conversational_message`,
  `detect_skips_already_scanned_message`, `detect_filters_maestro_targeted_changes`,
  `apply_directive_changes_writes_scope_through_governance`,
  `handoff_switches_to_workspace_and_clears_pending`).
* `scripts/quality-gate.sh` — **Quality gate passed** (5/5 stages, incl. doc link integrity).
* Architecture boundaries preserved: detection/apply/handoff live in the presentation TUI
  layer and delegate every write to `MarkdownGovernance`; no new domain/application leakage.
* Reused existing tested blocks (`parse_directive_proposals`, `apply_directive_change`,
  `registrations_from_governance`); the LLM path is additive and engine-gated.

# TASK 036: Wire Sequential Orchestration Into the Live Workspace Runtime

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Task 035 delivered a tested `AgentRuntime::orchestrate_sequential` capability, but the interactive Workspace Mode still starts the default personas as parallel broadcast agents and publishes user prompts directly to the `Environment`.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Execution_Plans/tasks/035-core-mode-directives-editor.md
* **Expected Output:** When a user submits a prompt in the interactive Workspace monitor, the Maestro agent drives the available agents through the sequential coordinator (`orchestrate_sequential`) with live narration and heartbeat, instead of the parallel broadcast path. The parallel runtime API stays intact for the headless `maestro run` cycle and onboarding, and the change is reversible via a safe fallback.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* The parallel broadcast runtime (`start_agents`, `run_agent_loop`, `process_message_cycle`) remains a supported API; its tests stay green and the headless `maestro run` cycle keeps using it.
* Only one sequential workflow may execute at a time (single-flight); concurrent user prompts are serialized, never interleaved on shared role state.
* Per-agent failure isolation from task 010 is preserved: a failing agent is recorded and the workflow continues.
* Narration and heartbeat remain timer-based and deterministic; no LLM-driven synthesis is introduced.
* The Maestro persona stays immutable; this task changes runtime orchestration only, not governance.
* No RAG or KV-cache behavior changes are in scope.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`, CLI boundary uses `anyhow`.

## 3. ACCEPTANCE CRITERIA
* AC1: `AgentRuntime` can store a sequential pipeline (`set_sequential_pipeline`) and report whether one is present (`has_sequential_pipeline`); storing a pipeline seeds each agent's health to `Idle` so the Agent Activity panel lists them before the first prompt.
* AC2: `AgentRuntime::orchestrate_user_message` runs the stored pipeline through `orchestrate_sequential` and is single-flight: two concurrent invocations are serialized so no more than one agent cycle is active at any time.
* AC3: The interactive Workspace bootstrap registers the default personas as a sequential pipeline (not parallel agents); a user prompt in Workspace Mode triggers `orchestrate_user_message`, falling back to direct `Environment` publish only when no sequential pipeline is present.
* AC4: All quality gates pass with added/updated tests covering AC1 and AC2.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`

## 5. INCREMENT MAP
* INC1: Runtime sequential-pipeline storage, health seeding, and single-flight `orchestrate_user_message` (`src/application/agent_runtime.rs`). Covers AC1 and AC2. (done)
* INC2: CLI bootstrap registers the Workspace pipeline and the TUI prompt path drives sequential orchestration with fallback (`src/presentation/cli/mod.rs`, `src/presentation/tui/mod.rs`). Covers AC3. (done)
* INC3: Quality gate run, doc updates, and evidence capture. Covers AC4. (done)

## 5b. VALIDATION EVIDENCE
* `cargo fmt --all -- --check`: clean (no diff).
* `cargo clippy --all-targets -- -D warnings`: clean (no warnings).
* `cargo test --all-targets`: 136 passed; 0 failed (adds AC1 pipeline-seeding/presence, AC2 stored-pipeline run, and AC2 single-flight concurrency tests; existing parallel-runtime tests remain green).
* `scripts/quality-gate.sh`: passed (fmt check, cargo check, clippy, workspace tests, doc-link integrity).

## 6. RESIDUAL RISKS
* Interactive TUI behavior is not unit-tested end to end; mitigate by unit-testing the runtime seam and keeping a parallel-publish fallback.
* Sharing role state across serialized prompts relies on single-flight; mitigate with a concurrency test asserting max active cycle count of one.

# TASK 039: Maestro Meta-Orchestrator (Plan → Delegate → Audit → Deliver)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** The interactive Workspace runtime currently runs a flat relay (`orchestrate_sequential`): Maestro is just agent #0 in a chain, each agent's output is piped to the next, and `PersonaRuntimeRole::act` appends a round-robin "Handoff to X" string from the interaction matrix. Maestro never plans, delegates, audits, or synthesizes — it does not orchestrate.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:src/application/agent_runtime.rs, #file:src/application/persona_operations.rs
* **Expected Output:** Maestro becomes a genuine meta-orchestrator. For a user demand it (1) **plans** a brief, (2) **delegates** the demand to each worker persona, (3) **audits** every worker's contribution, and (4) **delivers** a synthesized result plus an audit trail. The round-robin relay handoff is retired. Single-flight serialization and per-worker failure isolation are preserved.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Maestro is the orchestrator, never a relay node; workers are the non-Maestro personas.
* Preserve single-flight orchestration (`orchestration_lock`) and per-worker failure isolation (a failing worker is recorded and the workflow continues).
* No LLM calls in narration or heartbeat; heartbeats stay timer-based. Worker cognition continues to use the worker's own LLM via its role cycle.
* The runtime stays persona-agnostic: orchestration logic in `agent_runtime.rs` must not depend on concrete persona fields; delegation framing is generic.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`.
* No RAG or KV-cache behavior changes.

## 3. ACCEPTANCE CRITERIA
* AC1: A new `orchestrate_as_maestro` orchestrates plan → delegate (one per worker) → audit (per worker) → deliver, returning a report with a planning brief, per-worker delegation outcomes with audit verdicts, and a synthesized final delivery.
* AC2: Every worker receives the user demand directly (fan-out from Maestro), not a chained relay output; the round-robin "Handoff to X" relay in `PersonaRuntimeRole::act` is removed.
* AC3: A failing worker is isolated (audit verdict `Rejected`, workflow continues, health `Failed`); successful workers are audited `Approved` and included in the delivery.
* AC4: `orchestrate_user_message` drives the meta-orchestrator (splitting the stored pipeline into Maestro + workers) and stays single-flight; the flat relay `orchestrate_sequential` is removed.
* AC5: Maestro narrates `plan`, `delegate`, `audit`, and `deliver` phases as `MaestroNarration` events for the Workspace monitor.
* AC6: All quality gates pass.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1 (done): Retire the relay in `PersonaRuntimeRole::act` (drop `handoff_index`, return clean worker output) (`src/application/persona_operations.rs`). Covers AC2 (partial).
* INC2 (done): Add `orchestrate_as_maestro` + report types (`MaestroOrchestrationReport`, `MaestroDelegation`, `AuditVerdict`) with plan/delegate/audit/deliver phases and narration; remove `orchestrate_sequential` + `SequentialRunReport` (`src/application/agent_runtime.rs`). Covers AC1, AC2, AC3, AC5.
* INC3 (done): Rewire `orchestrate_user_message` to split the pipeline (`split_orchestrator`) and call `orchestrate_as_maestro`, preserving single-flight (`src/application/agent_runtime.rs`); update tests. Covers AC4.
* INC4 (done): Quality gate run, doc updates, and evidence capture. Covers AC6.

## 5b. VALIDATION EVIDENCE
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test --all-targets`: 145 passed, 0 failed (replaced 4 relay tests with 4 meta-orchestrator tests: delegate+audit+deliver, failing-worker isolation+rejection, slow-worker heartbeat, plan/delegate/audit/deliver narration; updated `orchestrate_user_message` tests for split-orchestrator semantics and single-flight).
* `bash scripts/quality-gate.sh`: passed (fmt --check, cargo check, clippy, test, doc link integrity).
* Behavior: Maestro now plans a brief, fans the demand out to each worker, audits each contribution (`Approved`/`Rejected`), and synthesizes a final delivery with an audit trail; the round-robin "Handoff to X" relay and `orchestrate_sequential` are removed. Single-flight and per-worker failure isolation preserved.

## 6. RESIDUAL RISKS
* Deterministic audit/synthesis keeps behavior testable but is intentionally simpler than LLM-based auditing; richer LLM-driven audit can be layered later without changing the orchestration contract.

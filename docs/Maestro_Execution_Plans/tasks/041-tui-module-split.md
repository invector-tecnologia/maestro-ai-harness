# TASK 041: Split the TUI God-Module into Cohesive Submodules

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** `src/presentation/tui/mod.rs` is a ~3900-line god-module mixing app state, the async run loop, all rendering, the creation wizard, the Architect picker, telemetry, async interview/directive helpers, and ~1000 lines of tests. This hurts readability, navigation, and SOLID single-responsibility.
* **Context Anchors:** #file:src/presentation/tui/mod.rs, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** The TUI presentation module is split into cohesive child submodules under `src/presentation/tui/` with **zero behavior change**. `mod.rs` retains `TuiApp`, its `impl`, the `run_tui` loop, public API, and shared small helpers; cohesive concerns move to dedicated files. All public re-exports are preserved so callers are unaffected.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Zero behavior change: no logic edits during the move; pure relocation plus visibility adjustments.
* Public API unchanged: `TuiApp`, `run_tui`, `OnboardingBootstrap`, `UIMode`, `AgentView`, `WizardSubmission` remain exported from `crate::presentation::tui`.
* Submodules are children of `tui`, so they may access `TuiApp` private fields; moved items get `pub(super)`/`pub(crate)` only as needed for cross-module calls.
* No production `unwrap()`/`expect()`/`panic!()` introduced; `tracing` for observability; no `println!`/`dbg!`.
* No RAG, KV-cache, orchestration, or Architect/Interview behavior changes.

## 3. ACCEPTANCE CRITERIA
* AC1: `src/presentation/tui/architect.rs` holds `DirectiveGroup`, `ArchitectEntry`, `ArchitectPicker`, and directive helpers. (done)
* AC2: `src/presentation/tui/wizard.rs` holds `WizardSubmission`, `WizardAdvance`, `WizardKind`, `WizardField`, `CreationWizard`, `persist_submission`, `slug`. (done)
* AC3: `src/presentation/tui/render.rs` holds the rendering functions and rendering-only helpers. (done)
* AC4: `src/presentation/tui/interview.rs` holds the async interview/directive helpers; `src/presentation/tui/telemetry.rs` holds telemetry helpers. (done)
* AC5: The `#[cfg(test)] mod tests` block moves to `src/presentation/tui/tests.rs`. (done)
* AC6: `mod.rs` is materially smaller (under ~1300 lines) and all quality gates pass with the same test count. (done — 642 lines, 146 tests)

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1: Extract `architect.rs` (no `TuiApp` coupling). (done)
* INC2: Extract `wizard.rs` (self-contained). (done)
* INC3: Extract `telemetry.rs`. (done)
* INC4: Extract `render.rs` (reads `&TuiApp`). (done)
* INC5: Extract `interview.rs` (async helpers). (done)
* INC6: Move tests to `tests.rs`; declare submodules in `mod.rs`; quality gate + evidence. (done)

## 5b. VALIDATION EVIDENCE
* `src/presentation/tui/mod.rs` reduced from ~3935 lines to 642 lines. New submodules: `architect.rs` (139), `wizard.rs` (339), `render.rs` (579), `interview.rs` (392), `telemetry.rs` (44), `tests.rs` (1002), `app.rs` (793, holds `impl TuiApp`).
* Pure relocation plus visibility adjustments: moved items raised to `pub(super)` only where cross-module calls require it; `WizardSubmission` re-exported via `pub use wizard::WizardSubmission;` to preserve the public API surface of `crate::presentation::tui`.
* `cargo fmt --all -- --check`: passed.
* `cargo clippy --all-targets -- -D warnings`: passed (0 warnings).
* `cargo test --all-targets`: 146 passed; 0 failed (same count as Task 040).
* `scripts/quality-gate.sh`: all 5 steps passed (fmt, check, clippy, test, doc links).
* `scripts/check-doc-links.sh`: documentation link integrity passed.
* Public API unchanged: `TuiApp`, `run_tui`, `OnboardingBootstrap`, `UIMode`, `AgentView`, `WizardSubmission` remain exported from `crate::presentation::tui`; zero behavior change confirmed by the unchanged passing test suite.

## 6. RESIDUAL RISKS
* Cross-module privacy: submodules rely on Rust's descendant-access rule for `TuiApp` privates; validated by full compile + clippy.
* Large mechanical move; correctness is enforced by the unchanged test suite passing with the same count.

# TASK 040: Mode Naming Consolidation (Architect Mode)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** The TUI's directive-governance screen is named inconsistently across code and docs: "Core Mode", "Core (Architect's) Mode", "Directives editor", and "Interview Mode — directive governance" all refer to the same picker. It is reached by two redundant commands (`/core` and `/edit`) and is implemented as an implicit sub-stage of `UIMode::Interview` (`mode == Interview && core_picker.is_some()`), overloading the same enum variant used for the conversational authoring/onboarding interview. README step 5 already calls this step **ARCHITECT**.
* **Context Anchors:** #file:src/presentation/tui/mod.rs, #file:docs/User_Manual/COMMANDS_AND_PANELS.md, #file:README.md
* **Expected Output:** One canonical name — **Architect Mode** — for the directive-governance picker, with one canonical command `/architect`. The picker is formalized as its own `UIMode::Architect` variant (no longer overloading `UIMode::Interview`). `UIMode::Interview` is reserved strictly for the guided Q&A authoring/onboarding session. The redundant `/edit` command is removed; `/core` is kept as a documented back-compat alias. All user-facing strings, in-app help, and docs use the single "Architect Mode" / `/architect` naming.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* No behavior change to the directive-governance flow itself (same picker → author transition, same Maestro immutability, same Esc-to-Workspace).
* `UIMode::Architect` is the picker (select stage); `UIMode::Interview` remains the authoring/onboarding Q&A; `UIMode::Workspace` and `UIMode::HelpMenu` unchanged.
* Exactly one advertised command (`/architect`); `/core` retained only as an undocumented-in-help back-compat alias; `/edit` removed.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; no `println!`/`dbg!`.
* No RAG, KV-cache, or orchestration-runtime behavior changes.
* Keep all docs/guides/manuals/readmes synchronized with the new naming.

## 3. ACCEPTANCE CRITERIA
* AC1: `UIMode` gains an `Architect` variant; the directive picker stage is dispatched by `mode == UIMode::Architect` (key handling and render), not by `mode == Interview && core_picker.is_some()`.
* AC2: `/architect` enters Architect Mode; `/core` still works as a back-compat alias; `/edit` is no longer a command.
* AC3: All user-facing strings (panel titles, headers, log lines, in-app `/help`) use "Architect Mode" — no remaining "Core Mode" / "Core (Architect's) Mode" / "Interview Mode — directive governance" naming for the picker.
* AC4: Internal identifiers for the picker are renamed for coherence (`CorePicker`→`ArchitectPicker`, `CoreEntry`→`ArchitectEntry`, `core_picker`→`architect_picker`, `core_selection_target`→`architect_selection_target`, `render_core_panel`→`render_architect_panel`).
* AC5: Docs (`README.md`, `docs/User_Manual/COMMANDS_AND_PANELS.md`) describe Architect Mode and `/architect` consistently; the `maestro directives` CLI is described as opening Architect Mode.
* AC6: All quality gates pass.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1 (done): Add `UIMode::Architect`; switch the picker key-handler and render dispatch to `mode == UIMode::Architect`; set it in `enter_directive_select` (`src/presentation/tui/mod.rs`). Covers AC1.
* INC2 (done): Make `/architect` canonical, keep `/core` alias, remove `/edit`; update in-app `/help` text (`src/presentation/tui/mod.rs`). Covers AC2.
* INC3 (done): Replace all picker user-facing strings/doc-comments with "Architect Mode"; rename internal `Core*` identifiers to `Architect*` (`src/presentation/tui/mod.rs`, plus doc-comments in `persona.rs`, `persona_operations.rs`, `interview_bot.rs`, `cli/mod.rs`). Covers AC3, AC4.
* INC4 (done): Sync docs (`README.md`, `docs/User_Manual/COMMANDS_AND_PANELS.md`, `docs/Practical_Guides/USER_ONBOARDING.md`, `docs/Practical_Guides/PROJECT_ONBOARDING.md`); add a test asserting `/architect` and `/core` enter `UIMode::Architect` and `/edit` does not. Quality gate + evidence. Covers AC5, AC6.

## 5b. VALIDATION EVIDENCE
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test --all-targets`: 146 passed, 0 failed (added `slash_commands_route_to_architect_mode`; renamed `architect_picker_*`, `architect_selection_target_*`, and `architect_command_*` tests assert `UIMode::Architect`).
* `bash scripts/quality-gate.sh`: passed (fmt --check, cargo check, clippy, test, doc link integrity).
* Naming: the directive-governance picker is now a dedicated `UIMode::Architect` variant; all user-facing strings, in-app `/help`, and docs use "Architect Mode" / `/architect`. `/core` retained as a back-compat alias; `/edit` removed. Internal `CorePicker`/`CoreEntry`/`core_picker`/`core_selection_target`/`render_core_panel` renamed to `Architect*`.

## 6. RESIDUAL RISKS
* `/core` is retained as a silent alias; a future release may drop it once muscle memory migrates to `/architect`.
* Internal `core_*` rename is mechanical; the language server rename keeps references consistent, validated by the full test + clippy gates.

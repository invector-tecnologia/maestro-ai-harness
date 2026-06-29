# TASK 035: Core Mode and Interview Directives Editor

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current onboarding-only Interview Mode, form-based CreationWizard, and markdown governance for personas, persona skills, and project scopes.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A Core Mode (Architect's) directives hub that launches Interview Mode as the single guided editor for Create/Edit/Update/Delete of personas, persona skills, and project scopes, with the Maestro persona ruling all operations and remaining immutable.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Maestro persona is immutable: it cannot be edited, updated, deleted, archived, or corrupted at any layer.
* Maestro persona is the only orchestrator that invokes persona, skill, and scope create/update operations.
* Delete is soft delete only: directives are archived under `maestro/archive/<type>/` preserving file name and scope numbering (gaps allowed; no renumber).
* No default landing mode: Core Mode and Interview Mode are intent-driven; Workspace remains the runtime monitor.
* Maestro AI Harness stays language-agnostic and architecture-first; companion specialization stays task-scoped.
* No RAG or KV-cache behavior changes are in scope for this task.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`, CLI boundary uses `anyhow`.

## 3. ACCEPTANCE CRITERIA
* AC1: Markdown governance exposes list, read, and archive operations for personas, persona skills, and scopes, and rejects any mutation or archive that targets the Maestro persona or its skills.
* AC2: Interview Mode supports an explicit operation model (Create, Edit, Update, Delete) and a directive target (Persona, Persona Skill, Project Scope), and Edit/Update load existing directive content.
* AC3: Maestro persona mutation intent is rejected in the interview and presentation layers, with a clear immutable-persona message and no filesystem change.
* AC4: The approval apply-path performs create as a new draft, edit/update as an overwrite of the existing directive, and delete as an archive move.
* AC5: Core Mode (`UIMode::Core`) renders an interactive directives picker grouped by type, shows Maestro as read-only, and launches Interview Mode with the selected operation and target.
* AC6: The default persona catalog seeds exactly Maestro, Project Manager, Quality Assurance, User Experience, and Software Engineer, each with default persona instructions and default skills (Software Engineer skills are language-agnostic), and the catalog passes persona validation.
* AC7: The form-based CreationWizard path is folded into the interview-driven editor so directive authoring has a single canonical path.
* AC8: A CLI entry launches Core Mode and returns a dedicated CLI outcome.
* AC9: All quality gates pass with added/updated tests covering AC1 through AC8.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`

## 5. INCREMENT MAP
* INC1 (done): Governance list/read/archive APIs and Maestro immutability guard (`src/application/markdown_governance.rs`). Covers AC1.
* INC2 (done): Default persona catalog rename and Maestro expertise/responsibilities (`src/application/persona.rs`, `maestro/personas/maestro.md`, scaffolding in `src/presentation/cli/mod.rs`). Covers AC6.
* INC3: Interview operation/target model and per-operation flows (`src/application/interview_bot.rs`). Covers AC2 and part of AC3.
* INC4: Apply-path for create/overwrite/archive and presentation-layer immutability (`src/presentation/tui/mod.rs`). Covers AC3 and AC4.
* INC5: Core Mode picker and launch wiring (`src/presentation/tui/mod.rs`). Covers AC5.
* INC6: Fold CreationWizard into the interview-driven editor (`src/presentation/tui/mod.rs`). Covers AC7.
* INC7: CLI Core subcommand and outcome (`src/presentation/cli/mod.rs`). Covers AC8.
* INC8: Quality gate run and evidence capture. Covers AC9.

## 6. RESIDUAL RISKS
* TUI state machine complexity may grow; mitigate with focused unit tests per transition.
* Folding the wizard touches existing create flows; mitigate by preserving create behavior parity tests.
* Soft-delete archive leaves scope numbering gaps by design; documented as accepted behavior.

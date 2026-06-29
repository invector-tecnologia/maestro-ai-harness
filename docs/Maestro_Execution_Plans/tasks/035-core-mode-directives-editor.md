# TASK 035: Core Mode and Interview Directives Editor

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current onboarding-only Interview Mode, form-based CreationWizard, and markdown governance for personas, persona skills, and project scopes.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Two intent-driven modes. (1) Interview Mode is the single directive governance home: it opens on a directive picker (select stage), then guides Create/Edit/Update/Delete of personas, persona skills, and project scopes (author stage). The Maestro persona orchestrates authoring and stays immutable: the Project Manager agent writes scope files first, then Maestro reads the written scope to derive the additions each non-Maestro persona (Project Manager, Quality Assurance, User Experience, Software Engineer) needs. (2) Workspace Mode is a lean runtime monitor where the Maestro agent orchestrates a sequential agent workflow with live narration. The former standalone Core Mode is folded into Interview Mode's select stage.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Maestro persona is immutable: it cannot be edited, updated, deleted, archived, or corrupted at any layer.
* Maestro persona is the only orchestrator that invokes persona, skill, and scope create/update operations.
* There is exactly one directive governance mode (Interview Mode); the directives picker is its select stage, not a separate top-level mode.
* Scope authoring is Project-Manager-first: the Project Manager agent understands, organizes, and writes scope files before Maestro derives persona additions; Maestro derives additions by reading the written scope, not by pre-guessing.
* Persona additions derived from a scope target only non-Maestro personas (Project Manager, Quality Assurance, User Experience, Software Engineer); Maestro is never a target for new skills.
* Derivation of persona additions is heuristic and deterministic in this task; no LLM-driven synthesis is introduced here.
* Delete is soft delete only: directives are archived under `maestro/archive/<type>/` preserving file name and scope numbering (gaps allowed; no renumber).
* No default landing mode: Interview Mode and Workspace Mode are intent-driven; Workspace Mode is a runtime monitor and does not author directives.
* In Workspace Mode the Maestro agent orchestrates a sequential workflow: each agent waits for the previous agent to finish, and Maestro narrates progress in real time (heartbeat at least every 5 seconds while an agent runs longer than 5 seconds), preserving per-agent failure isolation.
* Maestro AI Harness stays language-agnostic and architecture-first; companion specialization stays task-scoped.
* No RAG or KV-cache behavior changes are in scope for this task.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`, CLI boundary uses `anyhow`.

## 3. ACCEPTANCE CRITERIA
* AC1: Markdown governance exposes list, read, and archive operations for personas, persona skills, and scopes, and rejects any mutation or archive that targets the Maestro persona or its skills.
* AC2: Interview Mode supports an explicit operation model (Create, Edit, Update, Delete) and a directive target (Persona, Persona Skill, Project Scope), and Edit/Update load existing directive content.
* AC3: Maestro persona mutation intent is rejected in the interview and presentation layers, with a clear immutable-persona message and no filesystem change.
* AC4: The approval apply-path performs create as a new draft, edit/update as an overwrite of the existing directive, and delete as an archive move.
* AC5: Interview Mode opens on a select stage that renders an interactive directives picker grouped by type with Maestro read-only; selecting an actionable directive advances to the author stage with the chosen operation and target; there is no separate top-level Core Mode (`UIMode::Core` is removed).
* AC6: The default persona catalog seeds exactly Maestro, Project Manager, Quality Assurance, User Experience, and Software Engineer, each with default persona instructions and default skills (Software Engineer skills are language-agnostic), and the catalog passes persona validation.
* AC7: The form-based CreationWizard path is folded into the interview-driven editor so directive authoring has a single canonical path.
* AC8: A CLI entry launches Interview Mode (directive governance) and returns a dedicated CLI outcome.
* AC9: For a scope authoring request, the Project Manager agent writes the scope file first, then Maestro reads that written scope and derives the additions each non-Maestro persona (Project Manager, Quality Assurance, User Experience, Software Engineer) needs; Maestro is never a derivation target.
* AC10: After authoring, Maestro audits project dependencies and surfaces the required next actions in the Workspace monitor (hand-off from Interview Mode to Workspace Mode).
* AC11: Workspace Mode panels have explicit, integrated roles and a defined flow (input -> orchestration -> agent activity -> readiness/actions) with deterministic focus transitions.
* AC12: In Workspace Mode the Maestro agent orchestrates available agents sequentially: an agent starts only after the previous agent finishes, and Maestro emits a real-time narration event per transition plus a heartbeat at least every 5 seconds while any agent runs longer than 5 seconds.
* AC13: All quality gates pass with added/updated tests covering AC1 through AC12.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`

## 5. INCREMENT MAP
* INC1 (done): Governance list/read/archive APIs and Maestro immutability guard (`src/application/markdown_governance.rs`). Covers AC1.
* INC2 (done): Default persona catalog rename and Maestro expertise/responsibilities (`src/application/persona.rs`, `maestro/personas/maestro.md`, scaffolding in `src/presentation/cli/mod.rs`). Covers AC6.
* INC3 (done): Interview operation/target model and per-operation flows (`src/application/interview_bot.rs`). Covers AC2 and part of AC3.
* INC4 (done): Apply-path for create/overwrite/archive and presentation-layer immutability (`src/presentation/tui/mod.rs`). Covers AC3 and AC4.
* INC5 (superseded by INC6): standalone Core Mode picker (`src/presentation/tui/mod.rs`).
* INC6: Fold the picker into Interview Mode as a select stage, remove the standalone Core Mode, and fold CreationWizard into the interview-driven editor (`src/presentation/tui/mod.rs`). Covers AC5 and AC7.
* INC7: Maestro authoring pipeline — Project Manager agent writes scope first, then Maestro reads the scope and derives Project Manager / Quality Assurance / User Experience / Software Engineer additions, then dependency audit hand-off to Workspace (`src/application/interview_bot.rs`, `src/application/persona_operations.rs`). Covers AC9 and AC10. (done)
* INC8: CLI entry launches Interview Mode governance with a dedicated outcome (`src/presentation/cli/mod.rs`). Covers AC8. (done)
* INC9: Workspace panel roles and focus-flow integration pass (`src/presentation/tui/mod.rs`). Covers AC11.
* INC10: Sequential Maestro-orchestrated agent workflow with real-time narration and 5-second heartbeat (`src/application/agent_runtime.rs`, `src/application/agent_observability.rs`). Covers AC12.
* INC11: Quality gate run and evidence capture. Covers AC13.

## 6. RESIDUAL RISKS
* TUI state machine complexity may grow; mitigate with focused unit tests per transition.
* Folding the wizard touches existing create flows; mitigate by preserving create behavior parity tests.
* Soft-delete archive leaves scope numbering gaps by design; documented as accepted behavior.

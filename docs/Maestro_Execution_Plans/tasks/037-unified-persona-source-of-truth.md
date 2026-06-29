# TASK 037: Unified Persona Source of Truth

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Personas exist in three divergent, non-interoperable representations: the hardcoded Rust catalog `PersonaCatalog::default_personas()` (drives the runtime), the markdown templates emitted by `scaffold_personas`/`scaffold_skills`, and the committed `maestro/personas/maestro.md`. There is no markdown↔`Persona` parser, so Core/Directives Mode edits never reach the runtime agents.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Execution_Plans/tasks/035-core-mode-directives-editor.md, #file:docs/Maestro_Execution_Plans/tasks/036-workspace-sequential-orchestration-wiring.md
* **Expected Output:** One canonical persona markdown schema with a tested parser that produces a validated `Persona`. The runtime persona catalog is built from governed markdown when present and valid, falling back to the in-code defaults otherwise, so editing a persona in Core Mode changes the actual runtime agent set. Committed persona files for Maestro, Project Manager, Quality Assurance, User Experience, and Software Engineer use the canonical schema.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* The Maestro persona stays immutable: it is parsed and orchestrates, but governance still rejects mutation/archive of Maestro.
* The runtime must never panic on malformed persona markdown; parse/validation failures degrade gracefully to the in-code default catalog and are surfaced via `tracing`.
* Persona content stays product-neutral and language-agnostic; no business-domain or programming-language lock-in in default personas.
* Backward compatibility: `PersonaCatalog::default_personas()` remains a supported API and the source of fallback truth; its tests stay green.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`, the CLI boundary uses `anyhow`.
* No RAG or KV-cache behavior changes are in scope.

## 3. ACCEPTANCE CRITERIA
* AC1: A canonical persona markdown schema is defined and a parser (`Persona::from_markdown`) produces a `Persona` from it, mapping the structured interaction matrix (target / collaboration contract / expected handoff) and returning a typed error on malformed input. Round-trip (`to_markdown` → `from_markdown`) preserves the persona.
* AC2: `PersonaCatalog::from_governance` builds the catalog from on-disk persona markdown, validates it, and the loader falls back to `default_personas()` when the governance directory is empty, missing, or invalid. Maestro is always present and immutable.
* AC3: The interactive Workspace bootstrap registers the catalog resolved from governance (so Core Mode edits drive the runtime), while onboarding and `maestro run` keep working.
* AC4: Committed canonical persona files exist for Maestro, Project Manager, Quality Assurance, User Experience, and Software Engineer, and `maestro scaffold-markdown` emits the same canonical schema.
* AC5: All quality gates pass with added/updated tests covering AC1 and AC2.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1: Canonical schema + `Persona::from_markdown`/`to_markdown` parser and typed `PersonaParseError` with round-trip tests (`src/application/persona.rs`). Covers AC1. (done)
* INC2: `PersonaCatalog::from_governance` loader with graceful fallback and immutability guarantee (`src/application/persona.rs`, `src/application/markdown_governance.rs`). Covers AC2. (done)
* INC3: Wire the resolved catalog into the Workspace bootstrap; reconcile committed persona files, scaffold templates, and governance validation to the canonical schema (`src/presentation/cli/mod.rs`, `src/application/markdown_governance.rs`, `maestro/personas/*`). Covers AC3 and AC4. (done)
* INC4: Quality gate run, doc updates, and evidence capture. Covers AC5. (done)

## 5b. VALIDATION EVIDENCE
* `cargo fmt --all -- --check`: clean (no diff).
* `cargo clippy --all-targets -- -D warnings`: clean (no warnings).
* `cargo test --all-targets`: 144 passed; 0 failed (adds AC1 round-trip/parse/error tests and AC2 governance-load/fallback/immutability-override tests; updated governance persona-schema tests).
* `scripts/quality-gate.sh`: passed (fmt check, cargo check, clippy, workspace tests, doc-link integrity).
* Committed canonical persona files: `maestro/personas/{maestro,project-manager,quality-assurance,user-experience,software-engineer}.md`, generated from the runtime catalog so `to_markdown` → `from_markdown` is lossless.

## 6. RESIDUAL RISKS
* A hand-edited persona file may omit a required section; mitigate with typed parse errors and fallback to defaults, never a panic.
* Governance persona validation and the canonical schema must stay aligned; mitigate by deriving both from the same section vocabulary and testing the round-trip.

# TASK 038: Default Persona Skills and Project Scope

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Task 037 unified the persona source of truth, but `maestro/skills/` and `maestro/scopes/` ship empty. `scaffold_skills`/`scaffold_scope` can generate defaults, yet the bundled workspace has no committed skill or scope artifacts and no regression test proving the shipped defaults pass governance validation.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Execution_Plans/tasks/037-unified-persona-source-of-truth.md
* **Expected Output:** The bundled workspace ships a complete, governance-valid set of default persona skills (one per persona) and a default project scope. `maestro scaffold-markdown` emits the same artifacts idempotently, and a regression test asserts every shipped non-Maestro skill and the default scope pass governance validation.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* The Maestro persona stays immutable: its skill is shipped read-only and governance still rejects Maestro skill mutation.
* Skill content uses the canonical skill schema (`## Objective`, `## Triggers`, `## Inputs`, `## Outputs`, `## Constraints`); the scope uses the canonical scope schema (`## Objective`, `## Business Scope`, `## Deliverables`, `## Acceptance Criteria`, `## Dependencies`).
* Skills and scope content stay product-neutral and language-agnostic; no business-domain or programming-language lock-in.
* Scaffolding remains idempotent: existing files are never overwritten.
* No RAG or KV-cache behavior changes are in scope.
* Rust production paths must not use `unwrap()`, `expect()`, or `panic!()`; domain/application errors use `thiserror`, the CLI boundary uses `anyhow`.

## 3. ACCEPTANCE CRITERIA
* AC1: Committed default skill files exist for every persona under `maestro/skills/<slug>/`, each using the canonical skill schema.
* AC2: A committed default project scope exists under `maestro/scopes/` using the canonical scope schema and a valid sequence number.
* AC3: `maestro scaffold-markdown` emits the same default skills and scope, and a regression test asserts every generated non-Maestro skill and the generated scope pass governance validation.
* AC4: All quality gates pass with the added test.

## 4. VALIDATION COMMANDS
* `cargo fmt --all -- --check`
* `cargo clippy --all-targets -- -D warnings`
* `cargo test --all-targets`
* `scripts/quality-gate.sh`
* `scripts/check-doc-links.sh`

## 5. INCREMENT MAP
* INC1 (done): Regression test that scaffolds into a temp workspace and validates every non-Maestro skill and the default scope against governance (`src/presentation/cli/mod.rs`). Covers AC3.
* INC2 (done): Commit the default skill files and project scope to the bundled workspace (`maestro/skills/*`, `maestro/scopes/*`). Covers AC1 and AC2.
* INC3 (done): Quality gate run, doc updates, and evidence capture. Covers AC4.

## 5b. VALIDATION EVIDENCE
* `cargo clippy --all-targets -- -D warnings`: clean.
* `cargo test --all-targets`: 145 passed, 0 failed (added `shipped_default_skills_and_scope_pass_governance_validation`).
* `bash scripts/quality-gate.sh`: passed (fmt --check, cargo check, clippy, test, doc link integrity).
* Bundled defaults committed: `maestro/skills/{maestro,project-manager,quality-assurance,software-engineer,user-experience}/*.md` and `maestro/scopes/001-First-Release.md`, all governance-valid (Maestro skill shipped read-only by immutability).

## 6. RESIDUAL RISKS
* Hand-edited skills may drift from the canonical schema; mitigate with the governance validation regression test and idempotent scaffolding.

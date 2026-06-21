# TASK 033: Harness and Project Dependency Domain Separation

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current CLI onboarding/readiness flow and project governance structure.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Distinct dependency ownership model with explicit checks for harness-level and project-level dependencies.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Harness dependencies and project dependencies must not be conflated.
* Dependency check feedback must be actionable and deterministic.
* Existing onboarding and init flows must remain backwards-compatible.

## 3. ACCEPTANCE CRITERIA
* AC1: Introduce a project dependency manifest under `maestro/project-deps.yaml` with schema validation.
* AC2: Add CLI dependency checks that can target harness deps, project deps, or both.
* AC3: `maestro init` and `maestro scaffold-markdown` scaffold a default `project-deps.yaml` when missing.
* AC4: Documentation explains the two dependency domains and the corresponding CLI commands.
* AC5: Parsing and execution tests cover positive and failure scenarios for project dependency checks.

## 4. VALIDATION COMMANDS
* `cargo fmt --check`
* `cargo clippy --all-targets --all-features -- -D warnings`
* `cargo test`

## 5. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Platform Engineer specializing in dependency governance.
Goal: Separate Maestro harness dependencies from project companion dependencies.

Before generating code, open a `<reasoning>` block and validate behavior in both local and packaged environments.

Execute:
1. Add explicit project dependency schema and validation.
2. Add CLI commands to check dependency domains independently.
3. Scaffold default project dependency manifest in init/scaffold flow.
4. Add tests and docs for operational clarity.

[Cohesion Mechanism]:
- Ensure dependency ownership is clear and observable from both CLI and onboarding.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

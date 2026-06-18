# TASK 016: Quality Gate and Release Candidate 0.1.0

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** All previous tasks completed.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** MLP 0.1.0 release candidate ready for acceptance.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Clippy must pass with zero warnings.
* Test suite must cover the multi-agent flow and TUI wizards.
* Acceptance checklist is mandatory.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as Quality Lead for a platform product.
Goal: Close release candidate 0.1.0 with production quality.

Before generating code, open a `<reasoning>` block and validate traceability between product requirements and test evidence.

Execute:
1. Consolidate unit, integration, and e2e tests.
2. Validate:
   - Complete multi-agent flow.
   - Guided persona/scope/skill creation.
   - Provider and model configuration.
   - Debian installation and uninstallation.
3. Run the quality gates:
   - `cargo fmt`
   - `cargo clippy --workspace --all-targets --all-features -D warnings`
   - `cargo test --workspace`
4. Generate the final acceptance checklist with evidence.

[Cohesion Mechanism]:
- Confirm all MLP requirements defined by the user are covered.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

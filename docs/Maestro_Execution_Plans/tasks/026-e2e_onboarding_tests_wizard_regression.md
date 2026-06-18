# TASK 026: E2E Onboarding Tests and Wizard Regression

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Existing onboarding flows and wizards.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Test coverage for new flows and regression.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Must cover the happy path and relevant failure cases.
* Must not break existing wizard tests.
* Tests must be deterministic.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Quality Engineer for TUI flows.
Goal: Expand test coverage for user/project onboarding and wizard regression.

Before generating code, open a `<reasoning>` block and evaluate flakiness risks.

Execute:
1. Create tests for first-run, skip, and redirect flows.
2. Create end-to-end tests for project onboarding.
3. Validate that persona/scope/skill wizards remain correct.
4. Ensure a stable suite in local CI.

[Cohesion Mechanism]:
- Confirm that onboarding evolution does not introduce functional regression.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

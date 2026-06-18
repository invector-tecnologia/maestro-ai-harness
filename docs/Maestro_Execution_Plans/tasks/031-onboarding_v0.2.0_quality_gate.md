# TASK 031: Onboarding Quality Gate v0.2.0

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Completed onboarding scope implementation.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Quality evidence to release RC v0.2.0.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* fmt, clippy, and test gates are mandatory with no warnings.
* Coverage must include both onboarding flows and the skip redirect.
* Acceptance checklist must include evidence.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as Quality Lead for an incremental release.
Goal: Validate quality for the onboarding scope before v0.2.0.

Before generating code, open a `<reasoning>` block and validate requirement-to-test traceability.

Execute:
1. Consolidate the onboarding test suite.
2. Run the mandatory gates (fmt, clippy, test).
3. Record evidence and any residual gaps.
4. Update the acceptance checklist.

[Cohesion Mechanism]:
- Confirm technical readiness before the release candidate.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

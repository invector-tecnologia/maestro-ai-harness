# TASK 030: Onboarding Error Recovery Hardening

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Onboarding flows and current error handling.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Onboarding resilient to operational failures.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* No panic or unwrap in onboarding flows.
* Error messages must be action-oriented.
* Recovery must preserve context.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Reliability Engineer for interactive UX.
Goal: Harden error handling and recovery in onboarding.

Before generating code, open a `<reasoning>` block and validate realistic failure scenarios.

Execute:
1. Review and type-classify onboarding errors.
2. Add recovery paths per stage.
3. Preserve progress where appropriate.
4. Cover failure scenarios with tests.

[Cohesion Mechanism]:
- Ensure continuity of experience even when external failures occur.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

# TASK 019: Skip-to-Project Redirect in User Onboarding

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Onboarding state machine.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A tested automatic fallback rule.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Skipping the intro must redirect immediately to project onboarding.
* No dead state between flows.
* Test coverage for this behavior is required.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Runtime Engineer focused on predictable UX.
Goal: Ensure automatic redirect from user onboarding to project onboarding on skip.

Before generating code, open a `<reasoning>` block and validate invalid states and forbidden transitions.

Execute:
1. Implement the explicit skip → project transition.
2. Block inconsistent transitions.
3. Add flow and regression tests.

[Cohesion Mechanism]:
- Ensure onboarding continuity with no friction after skip.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

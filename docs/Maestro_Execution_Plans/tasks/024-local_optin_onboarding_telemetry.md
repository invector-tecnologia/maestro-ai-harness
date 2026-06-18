# TASK 024: Local Opt-In Onboarding Telemetry

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Onboarding and runtime events.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Local opt-in collection of onboarding metrics.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Explicit user opt-in required.
* No PII or secrets in logs.
* Format must be simple and auditable.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Product Analytics Engineer with a privacy focus.
Goal: Measure onboarding effectiveness via local opt-in telemetry.

Before generating code, open a `<reasoning>` block and evaluate privacy risks.

Execute:
1. Define essential events (start, completion, abandonment, error per stage).
2. Persist events locally in a human-readable format.
3. Add opt-in/opt-out configuration.
4. Cover with schema and persistence tests.

[Cohesion Mechanism]:
- Make onboarding improvements data-driven without violating privacy.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

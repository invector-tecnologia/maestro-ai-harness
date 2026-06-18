# TASK 020: Onboarding State Machine and Resume

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Onboarding states, local session storage.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** A formal state machine with safe resume support.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* States and transitions must be explicitly typed.
* Local persistence must not expose secrets.
* Resume must be idempotent.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Software Architect for TUI guided flows.
Goal: Implement the onboarding state machine with resume support.

Before generating code, open a `<reasoning>` block and validate state invariants.

Execute:
1. Define the primary states and substates.
2. Define events, actions, and allowed transitions.
3. Persist a progress snapshot and support resume.
4. Cover with unit tests for each transition.

[Cohesion Mechanism]:
- Ensure deterministic behavior across application restarts.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

# TASK 023: Progressive Validation and Onboarding Checkpoints

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Config schema, markdown governance, onboarding state machine.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Incremental validations with safe checkpoints.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Invalid stages must not be persisted.
* Errors must always include a recommended action.
* Checkpoints must be recoverable.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Reliability Engineer for guided flows.
Goal: Implement progressive validation and checkpoints in onboarding.

Before generating code, open a `<reasoning>` block and validate intermittent failure scenarios.

Execute:
1. Add per-stage validation before advancing.
2. Persist a checkpoint only after successful validation.
3. Implement safe rollback on error.
4. Test invalid states and recovery scenarios.

[Cohesion Mechanism]:
- Ensure onboarding is robust in imperfect environments.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

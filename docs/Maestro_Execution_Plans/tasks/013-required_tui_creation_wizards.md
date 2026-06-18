# TASK 013: Required Persona, Scope, and Skill Creation Wizards

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Required markdown structure and TUI base.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** A guided, blocking creation flow for valid artifacts.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Creating an incomplete artifact is forbidden.
* Required field validation must run in real time.
* Persistence only occurs in the final valid state.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a UX Engineer specializing in guided flows.
Goal: Ensure safe artifact creation during Maestro onboarding.

Before generating code, open a `<reasoning>` block and analyze UX failure points that could allow validation bypass.

Execute:
1. Implement a persona creation wizard requiring:
   - Responsibility.
   - Deliverables.
   - Instructions.
   - Interaction matrix.
2. Implement a scope creation wizard requiring:
   - Numeric prefix and scope name.
   - Business scope.
   - Acceptance criteria.
3. Implement a skill creation wizard requiring:
   - Target persona.
   - Goal.
   - Inputs and outputs.
   - Constraints.
4. Require complete validation before persisting.
5. Add tests for the happy path and for each required-field block.

[Cohesion Mechanism]:
- Confirm there is no path that bypasses validation.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

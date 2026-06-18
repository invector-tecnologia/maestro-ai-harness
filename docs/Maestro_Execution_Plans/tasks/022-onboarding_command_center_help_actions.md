# TASK 022: Onboarding Command Center (Help and Quick Actions)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** TUI command handling, onboarding state.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Help and contextual navigation commands.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Commands must be consistent and predictable.
* No conflicts with existing commands.
* Error messages must be action-oriented.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a DX Engineer for command-driven interfaces.
Goal: Add an onboarding command center to Maestro.

Before generating code, open a `<reasoning>` block and validate interface discoverability.

Execute:
1. Create commands: `/help`, `/onboarding`, `/restart-onboarding`, `/status`.
2. Display contextual help per active stage.
3. Cover parsing and behavior with tests.

[Cohesion Mechanism]:
- Ensure that users can discover the flow without reading external documentation.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

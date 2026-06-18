# TASK 018: Project Guided Setup Onboarding in Ratatui

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Config loader, markdown governance, runtime/TUI.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A guided flow for setting up a new project.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Validation per stage is required.
* Transactional persistence per checkpoint.
* No UI dependency outside the TUI.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Platform UX Engineer for guided project setup.
Goal: Implement Maestro's new-project onboarding in Ratatui.

Before generating code, open a `<reasoning>` block and validate robustness in an environment without an active provider.

Execute:
1. Create stages: diagnosis, provider/model config, markdown scaffold, final validation.
2. Offer a fast mode (defaults) and a detailed mode (advanced fields).
3. Show a final summary with recommended next commands.
4. Ensure recovery from failures without closing the session.

[Cohesion Mechanism]:
- Confirm that a new project is operational at the end of the flow.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

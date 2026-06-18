# TASK 017: User First-Run Onboarding in Ratatui

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current TUI, local session state, user onboarding requirements.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A first-run introduction flow inside the TUI.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* 100% in Ratatui, no external prompts.
* Must allow completion or skip without blocking the session.
* Messages must be action-oriented and traceable via log.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Product Engineer focused on developer-tool onboarding.
Goal: Implement Maestro's first-run user onboarding in Ratatui.

Before generating code, open a `<reasoning>` block and validate the first-contact UX.

Execute:
1. Create a welcome screen/state for a new user.
2. Implement short feature introduction steps.
3. Allow the options: continue, skip, or view again later.
4. Persist onboarding state (not started / in progress / completed / skipped).

[Cohesion Mechanism]:
- Ensure that first-run reduces time-to-first-useful-action for the user.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

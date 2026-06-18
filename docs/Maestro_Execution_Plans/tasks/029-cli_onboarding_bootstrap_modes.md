# TASK 029: CLI Onboarding Bootstrap Modes

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Current CLI, TUI onboarding, config loader.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** CLI-controlled onboarding entry points.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Flags must be clear and unambiguous.
* Must be compatible with both automation and manual use.
* Exit codes must be consistent.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a CLI Platform Engineer.
Goal: Add onboarding bootstrap in fast and detailed modes.

Before generating code, open a `<reasoning>` block and validate the UX for both CI pipelines and human terminal use.

Execute:
1. Add flags/commands to start onboarding.
2. Offer a fast mode with safe defaults.
3. Offer a detailed mode with more controls.
4. Cover parsing and minimal execution with tests.

[Cohesion Mechanism]:
- Make onboarding easily triggered in different contexts.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

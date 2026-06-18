# TASK 021: Playful UX and Animations in Ratatui

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** TUI render loop, onboarding and runtime states.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A responsive, animated visual layer.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Animations must be tick-driven and must not block input.
* ASCII fallback for terminals with limited character support.
* No visual noise that hurts readability.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Terminal UX Engineer.
Goal: Make Maestro's onboarding and operation playful and animated in Ratatui.

Before generating code, open a `<reasoning>` block and evaluate performance and accessibility.

Execute:
1. Add progress and spinner components per state.
2. Create visual feedback for success, error, and pending states.
3. Include contextual messages per stage and persona.
4. Ensure compatibility with terminals lacking extended Unicode.

[Cohesion Mechanism]:
- Confirm that animations aid navigation without impeding productivity.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

# TASK 012: Usable Ratatui TUI Base

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Working multi-agent runtime.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** A daily-operational interface.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Interface must not block.
* Agent state updates must be real-time.
* Must be keyboard-driven.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Terminal-First Product Engineer.
Goal: Build a genuinely usable TUI for Maestro operation.

Before generating code, open a `<reasoning>` block and model the event flow between the runtime, rendering, and user input.

Execute:
1. Deliver a layout with:
   - Agent panel.
   - Message and event log.
   - Command input.
2. Show status per agent (`idle`, `observe`, `think`, `act`, `error`).
3. Integrate user command dispatch to the runtime.
4. Add tests for rendering and the basic input flow.

[Cohesion Mechanism]:
- Confirm that the interface supports a continuous operational session.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

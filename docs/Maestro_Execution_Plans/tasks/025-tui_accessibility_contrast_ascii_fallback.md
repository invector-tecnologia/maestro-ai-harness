# TASK 025: TUI Accessibility (Contrast and ASCII Fallback)

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** TUI visual components.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** An inclusive interface across different terminals.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Minimum acceptable contrast for both light and dark themes.
* ASCII fallback for unsupported symbols.
* Must not rely on color alone for feedback.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as an Accessibility Engineer for TUI.
Goal: Ensure accessibility and visual robustness of the Maestro interface.

Before generating code, open a `<reasoning>` block and validate readability on limited terminals.

Execute:
1. Review the color palette and strengthen contrast.
2. Implement ASCII fallbacks for animated elements and symbols.
3. Add complementary textual indicators.
4. Create minimal render validation tests.

[Cohesion Mechanism]:
- Preserve usability across all terminal profiles.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

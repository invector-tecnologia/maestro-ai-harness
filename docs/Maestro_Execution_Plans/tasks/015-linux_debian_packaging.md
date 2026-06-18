# TASK 015: Linux Debian Packaging

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Working CLI and TUI.
* **Context Anchors:** #file:docs/Maestro_Manifesto/CONVENTIONS.md, #file:docs/Maestro_Manifesto/ARCHITECTURE.md
* **Expected Output:** Reliable installation and uninstallation on Linux.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Installation must be simple and reproducible.
* Uninstallation must support both normal removal and purge.
* The process must work in a clean environment without hidden dependencies.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Linux Release Engineer.
Goal: Distribute Maestro as an installable application for Debian/Ubuntu.

Before generating code, open a `<reasoning>` block and model the complete install → run → remove cycle.

Execute:
1. Structure the Debian package for the binary and required assets.
2. Implement post-installation and removal scripts.
3. Define a configuration preservation policy for normal uninstall.
4. Define an optional total purge behavior.
5. Add a smoke-test guide for a clean environment.

[Cohesion Mechanism]:
- Confirm the install, run, and uninstall cycle leaves no unexpected residue.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

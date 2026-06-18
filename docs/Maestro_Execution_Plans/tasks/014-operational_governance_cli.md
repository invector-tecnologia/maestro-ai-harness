# TASK 014: Operational CLI and Governance Commands

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Config, runtime, TUI, and markdown structure.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A robust CLI for execution and diagnostics.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Exit codes must be consistent.
* Error messages must be action-oriented.
* Commands must be idempotent where applicable.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Platform Engineer for developer tooling.
Goal: Deliver Maestro's complete operational command surface.

Before generating code, open a `<reasoning>` block and evaluate the operational experience for both interactive use and CI automation.

Execute:
1. Implement the following commands:
   - `run`
   - `tui`
   - `validate-config`
   - `list-agents`
   - `doctor`
   - `scaffold-markdown`
2. Ensure integration with typed logging and error handling.
3. Add parsing and minimal execution tests per command.

[Cohesion Mechanism]:
- Confirm predictable output for CI/CD automation.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

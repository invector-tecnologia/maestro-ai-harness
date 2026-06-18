# TASK 028: Project Onboarding Guide Documentation

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Implemented guided setup flow.
* **Context Anchors:** #file:README.md
* **Expected Output:** Official guide for setting up a new project with Maestro.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Setup sequence must be verifiable.
* Must cover both fast and detailed modes.
* Must include expected validations and common errors.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Documentation Engineer for technical onboarding.
Goal: Document Maestro's new-project onboarding.

Before generating code, open a `<reasoning>` block and validate reproducibility in a clean environment.

Execute:
1. Document the guided setup stages.
2. Show the expected result at the end of the flow.
3. Include configuration examples and verification steps.
4. Include action-oriented troubleshooting.

[Cohesion Mechanism]:
- Ensure user autonomy in project configuration.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

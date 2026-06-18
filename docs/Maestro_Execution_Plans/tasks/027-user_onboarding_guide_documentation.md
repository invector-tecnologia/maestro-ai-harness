# TASK 027: User Onboarding Guide Documentation

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Implemented first-run flow.
* **Context Anchors:** #file:README.md
* **Expected Output:** Official user onboarding guide.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Language must be objective and task-oriented.
* Steps must be reproducible on Linux.
* Must cover skip and resume scenarios.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Technical Writer for a developer-first product.
Goal: Document Maestro's new-user onboarding.

Before generating code, open a `<reasoning>` block and validate clarity for beginners.

Execute:
1. Document the onboarding goal and prerequisites.
2. Explain each step and supporting commands.
3. Include skip and resume scenarios.
4. Include brief troubleshooting.

[Cohesion Mechanism]:
- Reduce dependence on manual support for first use.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

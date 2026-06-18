# TASK 005: Mandatory Product Markdown Structure

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Governance definitions from the user.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Structures and validations for `maestro_scopes`, `maestro_personas`, and `maestro_skills`.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* No partial artifact creation without validation.
* Mandatory numeric prefix convention for scopes.
* Required fields per document type.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Product Architect for multi-agent systems.
Goal: Institutionalize Maestro's markdown governance.

Before generating code, open a `<reasoning>` block and validate consistency between the directory structure, file schema, and artifact creation flow.

Execute:
1. Define the official structure:
   - `maestro_scopes/` with files named `001-Scope-Name.md`, `002-Scope-Name.md`, etc.
   - `maestro_personas/` with one file per persona.
   - `maestro_skills/` with subfolders per persona containing skill files.
2. Define required content contracts for each type:
   - Scope: goal, business scope, deliverables, acceptance criteria, dependencies.
   - Persona: responsibility, deliverables, instructions, interaction matrix, limits.
   - Skill: goal, triggers, inputs, outputs, constraints.
3. Implement schema validation for all three types.
4. Generate tests that accept valid documents and reject invalid ones.

[Cohesion Mechanism]:
- Confirm that the flow does not permit persisting an incomplete document.
- Confirm naming and structural consistency.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

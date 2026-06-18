# TASK 007: Persona Model and Interaction Matrix

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Requirements for Product, Engineering, UX, and DevOps personas.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** A reusable and validatable persona contract.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* A persona only exists if it has responsibility, deliverables, and an interaction matrix.
* Interactions between personas must be explicit.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as a Multi-Agent Systems Designer.
Goal: Define a persona model with explicit collaboration governance.

Before generating code, open a `<reasoning>` block and verify consistency between responsibilities, deliverables, and inter-persona interfaces.

Execute:
1. Define the persona structure containing:
   - Identity and purpose.
   - Responsibilities.
   - Deliverables.
   - Operational instructions.
   - Interaction matrix with other personas.
   - Quality criteria per persona.
2. Create defaults for:
   - Product.
   - Engineering.
   - UX.
   - DevOps.
3. Integrate validation with mandatory rules.
4. Add tests for:
   - Valid persona.
   - Persona missing the interaction matrix.
   - Persona with invalid interactions.

[Cohesion Mechanism]:
- Confirm that each persona has clearly defined responsibility boundaries.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

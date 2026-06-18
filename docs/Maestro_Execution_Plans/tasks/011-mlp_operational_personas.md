# TASK 011: MLP Operational Personas

## 1. TASK SIGNATURE (DSPy Architecture)
* **Inputs:** Persona model and multi-agent runtime.
* **Context Anchors:** #file:docs/Maestro_Manifesto/ARCHITECTURE.md, #file:docs/Maestro_Manifesto/CONVENTIONS.md
* **Expected Output:** Four default personas working together.

## 2. ABSOLUTE CONSTRAINTS (1.58-bit Constraint)
* Each persona must be independently configurable.
* Deliverables per persona must be observable in the flow.
* Interactions between personas must follow an explicit matrix.

## 3. EXECUTION PROMPT (Paste into Copilot Chat)
"""
Act as an Agent Orchestration Lead.
Goal: Activate the default personas for the MLP product.

Before generating code, open a `<reasoning>` block and verify separation of responsibilities across personas.

Execute:
1. Configure the default personas:
   - Product.
   - Engineering.
   - UX.
   - DevOps.
2. Define responsibilities, deliverables, and default interactions for each.
3. Connect each persona to the runtime with its own parameters.
4. Add collaboration tests between personas.

[Cohesion Mechanism]:
- Confirm clear separation of responsibilities and handoffs.

Return ONLY the modified code blocks in Markdown. No introduction.
"""

---
applyTo: "**/*"
description: "Use when preparing pull requests, reviewing merge readiness, validating CI evidence, and enforcing delivery governance in GitHub workflows."
---

# GitHub Delivery Gates

## Pull Request Requirements
1. Every PR must link a plan task or justify why no plan update is required.
2. Validation evidence must include executed commands and relevant outcomes.
3. Risks and rollback notes must be explicit for behavior-changing work.

## Merge Readiness Rules
- Required checklist items must be checked, not only present in the PR body.
- CI must pass all required quality gates before merge.
- Any known deviation from manifesto or conventions must be documented and approved.

## Minimum Evidence
- Local test evidence for impacted scope.
- Architecture boundary compliance confirmation.
- Acceptance criteria confirmation tied to specification.

# Contributing to Maestro AI Harness

Thank you for your interest in contributing to Maestro AI Harness! We welcome contributions of all kinds: bug reports, feature requests, documentation improvements, and code changes.

This document provides guidelines and instructions for contributing effectively.

## Code of Conduct

Please note that this project is governed by our [Code of Conduct](.github/CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code.

## How to Report a Bug

If you've found a bug, please help us by reporting it:

1. **Check existing issues** first to avoid duplicates
2. **Use the bug report template** when creating a new issue: [Report a Bug](/issues/new?template=bug_report.md)
3. **Provide clear details:**
   - What you expected to happen
   - What actually happened
   - Steps to reproduce the issue
   - Your environment (OS, Cargo version, Maestro version)
   - Relevant error messages or logs

The more detail you provide, the faster we can help!

## How to Request a Feature

We love hearing ideas for improving Maestro. To suggest a new feature:

1. **Check existing issues and discussions** to see if it's already been proposed
2. **Use the feature request template**: [Request a Feature](/issues/new?template=feature_request.md)
3. **Explain your use case:**
   - What problem does this solve?
   - How would you use this feature?
   - Are there alternative approaches you've considered?

Feature requests are reviewed regularly and prioritized based on community interest and alignment with Maestro's vision.

## Development Setup

To contribute code, you'll need to set up a development environment:

### Prerequisites

- **Rust 1.70+**: Install from [rustup.rs](https://rustup.rs/)
- **Git** and **GitHub CLI** (optional but recommended)
- **Linux/macOS**: Full support; WSL2 on Windows recommended

### Getting Started

1. **Fork the repository** on GitHub
2. **Clone your fork:**
   ```bash
   git clone https://github.com/<your-username>/maestro-ai-harness.git
   cd maestro-ai-harness
   ```
3. **Add upstream remote:**
   ```bash
   git remote add upstream https://github.com/invector-tecnologia/maestro-ai-harness.git
   ```
4. **Create a feature branch:**
   ```bash
   git checkout -b feature/your-feature-name
   ```

### Detailed Setup

For comprehensive development setup, environment configuration, and troubleshooting, see [Project Onboarding Guide](docs/Practical_Guides/PROJECT_ONBOARDING.md).

## Building and Testing

### Run Tests

```bash
# Run all tests
cargo test --all-targets

# Run tests with output
cargo test --all-targets -- --nocapture

# Run tests for a specific module
cargo test -p maestro_core
```

### Code Quality Checks

Before committing, ensure your code passes all quality gates:

```bash
# Format code
cargo fmt --all

# Run clippy linter
cargo clippy --all-targets -- -D warnings

# Run full quality gate
scripts/quality-gate.sh
```

### Build the Project

```bash
# Debug build
cargo build

# Release build
cargo build --release
```

## Coding Conventions

Maestro follows strict coding conventions to maintain safety, clarity, and consistency. **All contributions must adhere to these standards.**

### Key Rules (Zero Tolerance)

1. **Never use `unwrap()`, `expect()`, or `panic!()`** in production code paths
   - Always propagate errors using the `?` operator
   - Use `thiserror` for domain/application errors; `anyhow` only at CLI boundaries

2. **Async Safety**
   - For shared mutable state, use `Arc<tokio::sync::RwLock<T>>` or `Arc<tokio::sync::Mutex<T>>`
   - Never use `std::sync::Mutex` or blocking I/O inside async Tokio runtime paths

3. **Observability**
   - Use `tracing` crate for all logging (`tracing::info!`, `tracing::debug!`, `tracing::error!`)
   - **Forbidden:** `println!`, `dbg!`, or `eprintln!` in production code

4. **Testing**
   - All domain modules must include a `#[cfg(test)]` block with unit tests
   - Use `mockall` to mock infrastructure traits in tests

5. **Architecture Boundaries**
   - `src/domain/` — pure business logic, no I/O or provider SDK imports
   - `src/application/` — orchestration and use case coordination
   - `src/infrastructure/` — external adapters, providers, persistence
   - `src/presentation/` — CLI/TUI and user-facing parsing

See the complete [CONVENTIONS.md](docs/Maestro_Manifesto/CONVENTIONS.md) document for detailed guidelines.

## Spec-Driven Delivery

Maestro follows a **specification-first** development workflow:

## Spec-Driven Delivery

Maestro follows a **specification-first** development workflow:

1. **Link to an Execution Plan Task**
   - Every PR must link to a plan task in `docs/Maestro_Execution_Plans/tasks/`
   - If your contribution doesn't fit an existing task, create a new one first
   - Justification: This ensures all code changes are intentional and traced

2. **Define Acceptance Criteria**
   - Acceptance criteria must be specific and testable (not vague)
   - They guide implementation and validation

3. **Provide Validation Evidence**
   - Include output from `cargo test`, `cargo clippy`, and relevant commands
   - Document how you validated each acceptance criterion

See [Spec-Driven Delivery](docs/Maestro_Manifesto/README.md) for detailed guidance.

## Submitting a Pull Request

### Before You Start

1. **Create a task in `docs/Maestro_Execution_Plans/tasks/`** if one doesn't exist
2. **Ensure tests pass** and quality gates succeed locally
3. **Update documentation** if your changes affect user-facing behavior
4. **Rebase on latest `main`:**
   ```bash
   git fetch upstream
   git rebase upstream/main
   ```

### PR Checklist

When submitting your PR, use the [PR template](.github/PULL_REQUEST_TEMPLATE.md) and ensure:

- [ ] PR title clearly describes the change
- [ ] Links to relevant execution plan task (required)
- [ ] Acceptance criteria are listed and testable
- [ ] Validation evidence is provided (test output, command results)
- [ ] Architecture boundaries are respected
- [ ] No `unwrap()`, `expect()`, or `panic!()` introduced
- [ ] Tests added or updated for changed behavior
- [ ] Documentation updated (if applicable)
- [ ] Code formatted: `cargo fmt --all`
- [ ] Lint checks pass: `cargo clippy --all-targets -- -D warnings`
- [ ] All tests pass: `cargo test --all-targets`

### PR Review Process

1. **Automated Checks:** CI pipelines validate tests, linting, and quality gates
2. **Code Review:** Maintainers review for architecture compliance, conventions adherence, and clarity
3. **Iteration:** Address feedback and push updates to your branch
4. **Merge:** Once approved, a maintainer will merge your PR

### What to Expect

- **Initial Review:** 2-5 business days
- **Feedback Loop:** We'll suggest improvements if needed
- **Approval:** PRs meeting all criteria are merged promptly

## Documentation

Good documentation is as important as good code. When contributing:

- **Update README.md** if you change user-facing features
- **Update docs/** if you change architecture or workflows
- **Add inline comments** for complex logic
- **Use `tracing`** for runtime observability (not comments alone)

All user-facing documentation must be in **US English**.

## Security

If you discover a security vulnerability, **please do not open a public issue**. Instead, report it confidentially using the [SECURITY.md](/.github/SECURITY.md) policy.

## Getting Help

- **Questions?** Open a discussion in [GitHub Discussions](../../discussions)
- **Need setup help?** See [PROJECT_ONBOARDING.md](docs/Practical_Guides/PROJECT_ONBOARDING.md)
- **Architecture questions?** Refer to [ARCHITECTURE.md](docs/Maestro_Manifesto/ARCHITECTURE.md)
- **Conventions clarification?** Check [CONVENTIONS.md](docs/Maestro_Manifesto/CONVENTIONS.md)

---

Thank you for helping make Maestro AI Harness better! 🚀

## License

By contributing, you agree that your contributions will be licensed under the [GNU General Public License v3.0](LICENSE).

---

Thank you for helping make Maestro AI Harness better! 🚀

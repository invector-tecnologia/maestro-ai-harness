# Security Policy

## Reporting a Vulnerability

**Do not open public GitHub issues for security vulnerabilities.** This could expose the vulnerability to bad actors.

Instead, please report security issues privately using GitHub's [Security Advisory feature](https://github.com/invector-tecnologia/maestro-ai-harness/security/advisories):

1. Navigate to the [Security tab](https://github.com/invector-tecnologia/maestro-ai-harness/security) in the repository
2. Click "Report a vulnerability"
3. Fill in the vulnerability details, including:
   - Description of the vulnerability
   - Affected versions (if known)
   - Steps to reproduce (if applicable)
   - Potential impact
   - Suggested fix (if available)

Alternatively, you may email security concerns to the project maintainers (contact information available in repository metadata).

## What to Expect

- **Acknowledgment:** We will acknowledge receipt of your report within 5 business days
- **Assessment:** We will assess the vulnerability and determine affected versions
- **Response:** We will work to develop and test a fix
- **Disclosure:** We will coordinate responsible disclosure and notify affected users
- **Timeline:** We aim to release security patches within 30 days of a confirmed report (exceptions for critical issues or complex fixes)

## Supported Versions

| Version | Supported            |
|---------|----------------------|
| 0.2.x   | ✅ Yes               |
| 0.1.x   | ⚠️ Limited support   |
| < 0.1.0 | ❌ No                |

Security patches will be released for supported versions. Users on unsupported versions are encouraged to upgrade.

## Security Considerations for Contributors

When contributing to Maestro AI Harness, please keep these security principles in mind:

- **No Panics:** Never use `unwrap()`, `expect()`, or `panic!()` in production paths. Malformed input should be gracefully handled, not cause crashes.
- **Error Handling:** Use `thiserror` for domain/application errors and `anyhow` at presentation boundaries. Propagate errors using the `?` operator.
- **Input Validation:** Treat all external input (especially AI provider responses) as untrusted. Validate before use.
- **Credential Safety:** Never hardcode credentials, API keys, or secrets. Use secure configuration management.
- **Dependency Updates:** Keep dependencies current and review security advisories regularly.
- **Async Safety:** For shared state in Tokio, use `Arc<tokio::sync::RwLock<T>>` or `Arc<tokio::sync::Mutex<T>>` — never `std::sync::Mutex` in async contexts.

See [CONVENTIONS.md](../docs/Maestro_Manifesto/CONVENTIONS.md) for complete development standards.

## Acknowledgments

We appreciate the security research community's responsible disclosure practices. Security researchers who report vulnerabilities will be acknowledged in release notes (with permission).

## Further Reading

- [OWASP Secure Coding Practices](https://owasp.org/www-project-secure-coding-practices-quick-reference-guide/)
- [Rust Security Guidelines](https://anssi-fr.github.io/rust-guide/)
- [GitHub Security Best Practices](https://docs.github.com/en/code-security)

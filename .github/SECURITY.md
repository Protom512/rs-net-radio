# Security Policy

## Supported Versions

Security updates are applied to the following versions:

| Version | Supported |
| ------- | --------- |
| Latest  | ✅        |
| Other   | ❌        |

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly.

### How to Report

1. **Do NOT** create a public issue for the vulnerability
2. Send by github private vulnerability report
3. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if known)

### What Happens Next

1. We will acknowledge receipt of your report within 48 hours
2. We will investigate the vulnerability
3. We will provide a timeline for the fix
4. We will coordinate disclosure with you

### Disclosure Policy

- We will disclose vulnerabilities once a fix is released
- Credit will be given to reporters (unless requested otherwise)
- We will work with you to ensure responsible disclosure

## Security Best Practices

When working with this project:

- Keep dependencies up to date
- Review security advisories for dependencies
- Use `cargo audit` to check for known vulnerabilities
- Follow secure coding practices

```bash
# Check for security vulnerabilities in dependencies
cargo install cargo-audit
cargo audit
```

## Additional Resources

- [Rust Security Advisory Database](https://github.com/RustSec/advisory-db)
- [Cargo Audit Documentation](https://github.com/RustSec/cargo-audit)

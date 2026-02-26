# Security Policy

## Supported Versions

This project is currently **experimental** and not production-ready. Security updates are applied to the latest version only.

| Version | Supported          |
| ------- | ------------------ |
| latest  | :white_check_mark: |

## Reporting a Vulnerability

If you discover a security vulnerability in this project, please report it responsibly.

**Do not open a public issue for security vulnerabilities.**

Instead, please report vulnerabilities by:

1. Opening a private security advisory via GitHub's [Security Advisories](../../security/advisories/new) feature
2. Or contacting the maintainers directly

### What to Include

When reporting a vulnerability, please include:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact
- Any suggested fixes (optional)

### What to Expect

- Acknowledgment of your report within 48 hours
- An assessment of the vulnerability within 7 days
- Regular updates on the progress toward a fix
- Credit in the security advisory (unless you prefer to remain anonymous)

## Security Considerations

As an experimental verification tooling project, mamut-lab may interact with untrusted input during testing scenarios. Users should:

- Run experimental code in isolated environments
- Never use experimental features with production data
- Review code before execution in sensitive contexts

## Disclosure Policy

We follow coordinated disclosure practices. Once a fix is available, we will:

1. Release the patched version
2. Publish a security advisory
3. Credit the reporter (with permission)

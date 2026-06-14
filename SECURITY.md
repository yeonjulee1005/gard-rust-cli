# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | ✅        |

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**

Report vulnerabilities via email: **yeonju.lee1005@gmail.com**

Please include:
- A description of the vulnerability
- Steps to reproduce
- Potential impact
- Suggested fix (optional)

## Response SLA

| Severity | Initial Response | Fix Target |
|----------|-----------------|------------|
| Critical | 24 hours        | 72 hours   |
| High     | 48 hours        | 7 days     |
| Medium   | 7 days          | 30 days    |

## Scope

In scope:
- Supply chain attacks via gard's own dependencies
- False negatives in Tier 1/2/3 detection that allow malicious packages through
- Path traversal or code execution in Tier 3 source analysis
- Manifest tampering or signature bypass

Out of scope:
- Vulnerabilities in packages gard detects (report those to their maintainers)
- Denial of service via extremely large package directories

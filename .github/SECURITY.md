# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability in mNFTS, please report it responsibly through **GitHub Security Advisories**.

1. Go to the [Security Advisories page](../../security/advisories) for this repository.
2. Click "Report a vulnerability" and fill in the details.
3. The maintainers will acknowledge your report within 72 hours and work with you on a fix.

**Please do not open public issues for security vulnerabilities.** Public disclosure before a fix is available puts all users at risk.

## Scope

Security issues of particular interest include:

- Memory safety bugs in the Rust core or FFI boundary
- Path traversal or symlink attacks during file operations
- Denial of service via malformed NTFS images
- Privilege escalation through the FSKit extension

## Supported Versions

| Version | Supported |
|---------|-----------|
| v0.1.x  | Yes       |

Thank you for helping keep mNFTS safe.

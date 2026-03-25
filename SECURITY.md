# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.1.x   | Yes       |

## Reporting

Report security issues to the repository maintainer. Do not open public issues for security vulnerabilities.

## Scope

Goonj is a computation library with no network access, no file I/O (beyond what consumers provide), and no unsafe code. The primary attack surface is malformed input (NaN, infinity, extreme values) which is handled via validation and clamping.

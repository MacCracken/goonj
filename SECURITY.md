# Security Policy

## Supported Versions

| Version | Supported |
|---------|-----------|
| 2.0.x   | Yes       |

## Reporting

Report security issues to the repository maintainer. Do not open public issues
for security vulnerabilities.

## Scope

Goonj is a Cyrius computation library with no network access and no file I/O
beyond what consumers provide. The primary attack surface is malformed input
(NaN, infinity, extreme values), handled via validation and clamping. The
library performs manual memory management (`alloc` + `load64`/`store64`) with
named offset constants; buffer-size contracts are stated per struct/module.

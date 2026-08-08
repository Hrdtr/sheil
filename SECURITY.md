# Security Policy

## Supported Versions

Only the latest release receives security fixes. Always update to the newest
version before reporting a vulnerability.

| Version        | Supported          |
| -------------- | ------------------ |
| Latest release | :white_check_mark: |
| Older releases | :x:                |

## Reporting a Vulnerability

**Do not open a public issue.** Instead, report vulnerabilities privately via
GitHub Security Advisories:

1. Go to **[Security > Advisories > Report a vulnerability](https://github.com/Hrdtr/sheil/security/advisories/new)**.
2. Provide a clear description of the vulnerability, including:
   - Affected components (e.g., SSH, terminal emulator, credential storage)
   - Steps to reproduce
   - Potential impact
   - Suggested fix (if available)
3. A maintainer will acknowledge your report within **5 business days** and
   provide an initial assessment within **10 business days**.

## Scope

Security issues that qualify for private disclosure include (but are not
limited to):

- Credential leakage or insecure storage of SSH keys / passwords
- Remote code execution through the SSH protocol handler
- Privilege escalation within the Tauri application sandbox
- Man-in-the-middle vulnerabilities in transport encryption
- Session hijacking or unauthorized cross-session access

## Disclosure Timeline

1. Report is submitted privately.
2. Maintainer confirms receipt and triages severity.
3. A fix is developed and tested.
4. A security advisory is published alongside the patched release.
5. Public credit is given to the reporter (unless anonymity is requested).

## Scope Exclusions

The following are generally out of scope and may be reported as regular issues:

- Vulnerabilities in dependencies that have already been patched upstream
- Theoretical attacks requiring physical device access
- Social engineering or phishing attacks
- Denial-of-service through intentional resource exhaustion (rate-limiting is
  a feature request, not a vulnerability)

# Contributing to Sheil

Thank you for your interest in contributing! Sheil is an open-source, cross-platform SSH/telnet/serial client licensed under AGPLv3.

## Developer Certificate of Origin (DCO)

All contributors must sign off on the [Developer Certificate of Origin](https://developercertificate.org/) (DCO). This is a lightweight way to certify that you wrote or have the right to submit the code you are contributing.

By signing off, you certify the following:

```
Developer Certificate of Origin
Version 1.1

Copyright (C) 2004, 2006 The Linux Foundation and its contributors.

Everyone is permitted to copy and distribute verbatim copies of this
license document, but changing it is not allowed.

Developer's Certificate of Origin 1.1

By making a contribution to this project, I certify that:

(a) The contribution was created in whole or in part by me and I
    have the right to submit it under the open source license
    indicated in the file; or

(b) The contribution is based upon previous work that, to the best
    of my knowledge, is covered under an appropriate open source
    license and I have the right under that license to submit that
    work with modifications, whether created in whole or in part
    by me, under the same open source license (unless I am
    permitted to submit under a different license), as indicated
    in the file; or

(c) The contribution was provided directly to me by some other
    person who certified (a), (b) or (c) and I have not modified
    it.

(d) I understand and agree that this project and the contribution
    are public and that a record of the contribution (including all
    personal information I submit with it, including my sign-off) is
    maintained indefinitely and may be redistributed consistent with
    this project or the open source license(s) involved.
```

### How to Sign Off

Add a `Signed-off-by` line to every git commit message:

```bash
git commit -s -m "Your commit message"
```

Or manually append the line:

```
Signed-off-by: Your Name <your.email@example.com>
```

Commits without a sign-off will not be merged.

## Getting Started

### Prerequisites

- [Node.js](https://nodejs.org/) (managed via nub is recommended)
- [Rust](https://www.rust-lang.org/tools/install)

### Setup

```bash
nub install
```

### Development

```bash
nubx tauri dev
```

### Build

```bash
nubx tauri build
```

## Project Structure

```
sheil/
├── src/           # Svelte 5 frontend source
├── static/        # Static assets
├── tauri/         # Tauri 2 / Rust backend
└── package.json   # Workspace root
```

## Pull Request Process

1. Fork the repository and create a branch from `main`.
2. Ensure your code follows the existing style conventions.
3. Write or update tests as appropriate.
4. Ensure all tests pass: `nub test`
5. Sign off all commits (see DCO section above).
6. Submit a pull request with a clear description of the changes.
7. A maintainer will review your PR. Be responsive to feedback.

## Code of Conduct

- Be respectful and constructive in discussions.
- Focus on the technical merits of changes.
- Assume good faith from other contributors.

## Questions?

Open a [discussion](https://github.com/hrdtr/sheil/discussions) or ask in an issue.

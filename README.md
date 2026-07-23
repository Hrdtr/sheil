# Sheil

An open-source, cross-platform tool for managing remote and device connections with an embedded local AI engine for command generation and auto-completion.

## Features

- **Connection management**: Secure connections with key and password authentication
- **Tab-based interface**: Every tab is an active session with real-time state management
- **SFTP file management**: Integrated file browser with drag-and-drop upload/download
- **Port forwarding**: Local, remote, and dynamic (SOCKS) port forwarding
- **Local AI engine**: Embedded llama.cpp for terminal command generation and ghost-text completion
- **Encrypted credentials**: AES-256-GCM encryption for passwords and SSH keys at rest
- **SSH key management**: Import, store, and manage SSH keys with passphrase support
- **Host organization**: Group hosts, add tags, and import/export configurations
- **Cross-platform**: Built with Tauri 2 for macOS, Windows, and Linux
- **Modern UI**: Clean interface with dark/light mode

## Requirements

- **Node.js** >= 24.17.0
- **Rust** (2021 edition, MSRV 1.77.2)
- **pnpm** 11.x

## Setup

Install dependencies:

```bash
pnpm install
```

## Development

Start the development server (Tauri + Nuxt):

```bash
pnpm exec tauri dev
```

This launches the Nuxt dev server on `:3000` and opens the Tauri desktop app with hot-reload enabled.

### Frontend-only development

```bash
pnpm run dev
```

Runs the Nuxt dev server without Tauri (useful for UI work that doesn't require backend interaction).

## Build

Create a production build:

```bash
pnpm exec tauri build
```

Output binaries are placed in `tauri/target/release/bundle/`.

### Frontend-only build

```bash
pnpm run build
```

Generates static files to `dist/` (used by Tauri during the build process).

## Code Quality

### Formatting

```bash
pnpm run fmt          # Format with oxfmt
pnpm run fmt:check    # Check formatting
```

### Linting

```bash
pnpm run lint         # Lint with oxlint
```

### Type checking

```bash
pnpm run typecheck    # nuxi typecheck
```

### Testing

```bash
pnpm run test         # cargo test --manifest-path tauri/Cargo.toml
```

## Project Structure

```
sheil/
├── app/              # Nuxt 4 / Vue 3 frontend
│   ├── components/   # Vue components (terminal, hosts, settings, etc.)
│   ├── composables/  # Reactive state (sessions, hosts, AI engine, etc.)
│   └── utils/        # Tauri IPC wrappers and helpers
├── tauri/            # Tauri 2 / Rust backend
│   ├── src/
│   │   ├── commands/ # IPC handlers (ssh, sftp, hosts, port_forward, ai)
│   │   ├── db.rs     # SQLite initialization and migrations
│   │   ├── crypto.rs # AES-256-GCM encryption
│   │   └── secrets.rs # Credential storage
│   └── migrations/   # SQLx database migrations
└── package.json      # Workspace root
```

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) for:

- Developer Certificate of Origin (DCO) sign-off requirements
- Pull request process
- Code of conduct

All commits must be signed off with `git commit -s`.

## Tech Stack

**Frontend**: Nuxt 4, Vue 3, Tailwind CSS v4, xterm.js, VueUse  
**Backend**: Tauri 2, Rust, russh, sqlx, tokio, llama-cpp-2, aes-gcm  
**Tooling**: pnpm, oxfmt, oxlint, commitlint

## License

GPLv3 — see [LICENSE](LICENSE).

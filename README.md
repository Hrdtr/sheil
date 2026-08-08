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

## Download

Grab the latest release for your platform from the [releases page](https://github.com/Hrdtr/sheil/releases):

| Platform              | Asset                                              |
| --------------------- | -------------------------------------------------- |
| macOS (Apple Silicon) | `Sheil_<version>_aarch64.dmg`                      |
| macOS (Intel)         | `Sheil_<version>_x64.dmg`                          |
| Windows               | `Sheil_<version>_x64-setup.exe` or `.msi`          |
| Linux (x86_64)        | `.deb`, `.rpm`, or `.AppImage` (`amd64`/`x86_64`)  |
| Linux (ARM64)         | `.deb`, `.rpm`, or `.AppImage` (`arm64`/`aarch64`) |

## Development

Requirements: Node.js >= 24.17.0, Rust (MSRV 1.77.2), pnpm 11.x

```bash
pnpm install          # install dependencies
pnpm exec tauri dev   # run the app in dev mode
pnpm exec tauri build # production build
pnpm run lint         # lint
pnpm run typecheck    # type check
pnpm run test         # run Rust tests
```

See [AGENTS.md](AGENTS.md) for architecture details and coding conventions.

## Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) first. All commits must be signed off with `git commit -s`.

## License

GPLv3 — see [LICENSE](LICENSE).

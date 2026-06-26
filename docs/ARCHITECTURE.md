# Sheil Architecture

> **Reference:** [HRD-1](mention://issue/a027374d-e40a-428d-8a6e-ff786d76c5e6) — full research, trade-off analysis, and implementation plan.

Sheil is an open-source, cross-platform SSH/telnet/serial client — a Termius alternative. This document describes the architecture, technology decisions, and design rationale.

---

## Tech Stack

| Layer                      | Technology                                                                                                | Rationale                                                                                                                                                              |
| -------------------------- | --------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Desktop & mobile shell** | [Tauri v2](https://v2.tauri.app/)                                                                         | 10–100x smaller bundles than Electron, 5–8x less memory. Native mobile support (iOS, Android) since v2.0 (Oct 2024). Rust backend gives us performance-critical paths. |
| **Frontend framework**     | [Svelte 5](https://svelte.dev/)                                                                           | Zero runtime overhead — compiles to vanilla JS at build time, no virtual DOM. Smaller bundles than Vue or React. Runes ($state, $derived, $effect) for reactive state. |
| **UI components**          | [shadcn-svelte](https://www.shadcn-svelte.com/) + [Tailwind CSS 4](https://tailwindcss.com/)              | Copy-paste component library (not a dependency). Full control over markup and styling. Official Tauri integration docs available.                                      |
| **Terminal emulator**      | [xterm.js](https://xtermjs.org/) + `@xterm/addon-webgl` + `@xterm/addon-fit`                              | Powers VS Code's terminal. GPU-accelerated rendering via WebGL addon. DOM renderer fallback. Mature (MIT, 18K+ stars).                                                 |
| **SSH / SFTP**             | [russh](https://github.com/Eugeny/russh) 0.61 + [russh-sftp](https://crates.io/crates/russh-sftp)         | Pure Rust SSH implementation (Apache 2.0). Covers sessions, PTY, all auth methods, ciphers, keys, port forwarding, agent forwarding, and SFTP client/server.           |
| **Serial**                 | [serialport-rs](https://crates.io/crates/serialport) 4                                                    | Cross-platform serial port enumeration and I/O (USB CDC-ACM, FTDI, etc.).                                                                                              |
| **Telnet**                 | Custom Rust (TCP + option negotiation state machine)                                                      | ~200 LoC. Telnet is a trivial protocol; no heavyweight dependency needed.                                                                                              |
| **Local database**         | [SQLite](https://sqlite.org/) via [sqlx](https://crates.io/crates/sqlx) 0.8                               | Embedded, zero-config. Stores host configs, connection groups, tags, snippets, audit trail. Never stores credentials.                                                  |
| **Async runtime**          | [Tokio](https://tokio.rs/) 1                                                                              | Industry-standard async runtime for Rust. Required by russh, sqlx, and tauri-plugin-background-service.                                                                |
| **Secure storage**         | [tauri-plugin-keystore](https://crates.io/crates/tauri-plugin-keystore) 2.1                               | Wraps macOS Keychain, Windows Credential Manager, Linux Secret Service, iOS Keychain, Android Keystore. Passwords and private keys never touch SQLite.                 |
| **Background tasks**       | [tauri-plugin-background-service](https://crates.io/crates/tauri-plugin-background-service) 0.7           | Android foreground service + iOS BGTaskScheduler for SSH connection keepalive on mobile.                                                                               |
| **Logging**                | [tauri-plugin-log](https://crates.io/crates/tauri-plugin-log) 2 + [log](https://crates.io/crates/log) 0.4 | Structured logging with levels. Debug-only in dev; stripped from release.                                                                                              |
| **Serialization**          | [serde](https://serde.rs/) + [serde_json](https://crates.io/crates/serde_json)                            | Tauri IPC serialization, command argument/return types.                                                                                                                |
| **Error handling**         | [thiserror](https://crates.io/crates/thiserror) 2                                                         | Derive `Error` for typed error enums. Serde-compatible error serialization to frontend.                                                                                |

### Tooling

| Tool                                                                                  | Purpose                                                                                        |
| ------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| [nub.js](https://nubjs.com/)                                                          | Node.js version management and task runner (`nub install`, `nub run <script>`).                |
| [Vite](https://vite.dev/) 8                                                           | Frontend dev server and bundler. Configured for Tauri HMR and Svelte preprocessing.            |
| [oxfmt](https://oxc.rs/) + [oxlint](https://oxc.rs/)                                  | Rust-based formatter and linter. Faster alternatives to Prettier and ESLint.                   |
| [Vitest](https://vitest.dev/) 4                                                       | Frontend unit/integration tests. jsdom environment, mock Tauri IPC via `@tauri-apps/api/core`. |
| `cargo test`                                                                          | Rust unit tests with tempfile for I/O isolation.                                               |
| [commitlint](https://commitlint.js.org/) + [Husky](https://typicode.github.io/husky/) | Conventional commit enforcement via git hooks.                                                 |
| [lint-staged](https://github.com/lint-staged/lint-staged)                             | Run formatter/linter only on staged files.                                                     |

### License

[AGPLv3](https://www.gnu.org/licenses/agpl-3.0.en.html) — ensures all users of the software (including network users) have access to the source.

---

## Dependency Graph

```
┌──────────────────────────────────────────────────────────┐
│                    Frontend (Svelte 5)                   │
│  src/                                                    │
│  ├── main.ts            Entry point — mounts App.svelte  │
│  ├── App.svelte         Root component                   │
│  ├── app.css            Tailwind + shadcn theme          │
│  └── lib/                                                │
│      ├── commands.svelte.ts   Tauri IPC wrapper          │
│      ├── utils.ts             cn(), type helpers         │
│      └── components/                                     │
│          ├── TerminalView.svelte       xterm.js renderer │
│          ├── ConnectionManager.svelte  Host CRUD         │
│          ├── QuickConnect.svelte       One-click SSH     │
│          └── ui/              shadcn-svelte components   │
│              ├── button/                                 │
│              ├── card/                                   │
│              └── input/                                  │
├──────────────────────────────────────────────────────────┤
│  Dependencies                                            │
│  ├── @tauri-apps/api 2.11     IPC invoke() bridge        │
│  ├── xterm + addon-webgl      Terminal rendering         │
│  ├── @xterm/addon-fit         Responsive resize          │
│  ├── @xterm/addon-web-links   URL detection + click      │
│  ├── @lucide/svelte           Icon set                   │
│  ├── tailwindcss 4.3          Utility-first CSS          │
│  ├── tailwind-merge 3.6       Class deduplication        │
│  ├── clsx 2.1                 Conditional classes        │
│  └── tailwind-variants 3.2    Component variants         │
└──────────────┬───────────────────────────────────────────┘
               │  Tauri IPC (invoke / command)
               │  JSON serialization via serde
               ▼
┌──────────────────────────────────────────────────────────┐
│                    Rust Backend (Tauri 2)                │
│  tauri/src/                                              │
│  ├── main.rs           Windows subsystem + entry point   │
│  ├── lib.rs            Tauri builder, plugins, handlers  │
│  └── commands/                                           │
│      ├── mod.rs        Module declarations               │
│      ├── default.rs    read/write commands (placeholder) │
│      └── errors.rs     Typed error enum + serde format   │
├──────────────────────────────────────────────────────────┤
│  Tauri Plugins                                           │
│  ├── tauri-plugin-keystore     OS credential store       │
│  ├── tauri-plugin-log          Structured logging        │
│  └── tauri-plugin-bg-service   Mobile keepalive          │
├──────────────────────────────────────────────────────────┤
│  Rust Dependencies                                       │
│  ├── russh 0.61             SSH client (sessions, PTY,   │
│  │                            port forwarding, keys)     │
│  ├── russh-sftp 2           SFTP client/server           │
│  ├── serialport 4           Serial port I/O              │
│  ├── sqlx 0.8               SQLite (async, migrations)   │
│  ├── tokio 1                Async runtime                │
│  ├── serde + serde_json     Serialization                │
│  ├── thiserror 2            Error derivation             │
│  └── log 0.4                Logging facade               │
└──────────────────────────────────────────────────────────┘
```

### IPC Communication

All frontend-to-backend communication goes through Tauri's `invoke` → `#[tauri::command]` bridge:

1. **Frontend** calls `invoke('command_name', { arg1, arg2 })` via `@tauri-apps/api/core`
2. **Tauri IPC** deserializes arguments (JSON → Rust structs via serde)
3. **Command handler** executes on the Rust side (async via Tokio)
4. **Return value** serialized back to JSON, delivered to frontend

Commands are registered in `lib.rs` via `tauri::generate_handler![]`. Errors are serialized as `{ name: string, message: string }` for structured frontend error handling.

---

## Component Architecture

### Frontend (Svelte 5)

Svelte 5 uses **runes** — `$state()`, `$derived()`, `$effect()`, `$inspect()` — for reactive state management instead of Svelte 4's `let` / `$:` syntax.

**State pattern** (from `commands.svelte.ts`):

- `GlobalState` class encapsulates reactive state with `$state()` objects and exposes getter/setter accessors.
- Async `read()` and `write()` methods call Tauri `invoke()` and update local state on return.
- Components import and instantiate `new GlobalState()`, then bind to its properties with Svelte bindings.

**Component hierarchy** (planned):

```
App.svelte
├── TerminalView.svelte          xterm.js terminal instance
│   ├── xterm.js Terminal object
│   ├── WebGL addon (GPU renderer)
│   ├── Fit addon (responsive resize)
│   ├── WebLinks addon (URL detection)
│   └── Custom touch handler (mobile shim)
├── ConnectionManager.svelte     Host list, add/edit/delete
│   ├── Host form (name, host, port, auth method)
│   ├── Host list (grouped by tags/folders)
│   └── SQLite-backed via Tauri commands
├── QuickConnect.svelte          One-click host → terminal
├── Settings/Themes UI           Font, colors, cursor style
└── ui/                          shadcn-svelte primitives
    ├── Button
    ├── Card (+ Header, Title, Content)
    ├── Input
    └── (expandable: Dialog, Select, Tabs, etc.)
```

**Utility types** (`utils.ts`):

- `cn(...inputs)` — merges Tailwind classes with `clsx` + `tailwind-merge`
- `WithoutChild<T>`, `WithoutChildren<T>`, `WithoutChildrenOrChild<T>` — omit Svelte `child`/`children` props from shadcn-svelte component types for wrapping
- `WithElementRef<T, U>` — add `ref?: U` to component props for direct DOM access

### Backend (Rust)

```
lib.rs  ──  Tauri Builder
  ├── .plugin(tauri_plugin_keystore::init())
  ├── .plugin(init_with_service(|| PlaceholderService))
  ├── .setup(|app| { /* debug logging */ })
  └── .invoke_handler(generate_handler![read, write])

commands/
  ├── default.rs     Placeholder file I/O (will be replaced)
  │   ├── read(path) → String
  │   └── write(path, contents) → ()
  └── errors.rs      Error enum (Io, Utf8) with serde
```

**Planned backend expansion** (Phase 1+):

```
commands/
  ├── ssh.rs           SSH connection lifecycle
  │   ├── connect(host, port, auth) → SessionHandle
  │   ├── spawn_pty(session) → ChannelHandle
  │   ├── exec(session, cmd) → Output
  │   └── disconnect(session)
  ├── serial.rs        Serial port enumeration + I/O
  │   ├── list_ports() → Vec<PortInfo>
  │   └── connect(port, baud, config) → Stream
  ├── hosts.rs         Host configuration CRUD
  │   ├── list_hosts() → Vec<Host>
  │   ├── add_host(host) → Host
  │   ├── update_host(id, host)
  │   └── delete_host(id)
  ├── keystore.rs      Credential operations
  │   ├── store(name, key)
  │   ├── get(name) → Key
  │   └── delete(name)
  └── snippets.rs      Saved command management
```

---

## Security Model

### Credential Storage

```
┌─────────────────────────────────────┐
│  User enters password / imports key │
└──────────────┬──────────────────────┘
               ▼
┌─────────────────────────────────────┐
│  Frontend: invoke('store', { name,  │
│             key })                  │
└──────────────┬──────────────────────┘
               ▼
┌─────────────────────────────────────┐
│  tauri-plugin-keystore              │
│  ├── macOS: Keychain                │
│  ├── Windows: Credential Manager    │
│  ├── Linux: Secret Service (D-Bus)  │
│  ├── iOS: Keychain                  │
│  └── Android: Android Keystore      │
└─────────────────────────────────────┘
```

**Principles:**

1. **Credentials never touch SQLite.** Host configs (name, hostname, port, username) live in SQLite. Passwords and private keys are stored exclusively via `tauri-plugin-keystore` in the OS-native secure store.
2. **Keys never leave the backend.** The frontend sends credentials to Rust commands; the Rust side unwraps from the keystore and passes directly to `russh` for authentication. The frontend never sees raw key material.
3. **Import only.** MVP (Phase 1) supports importing existing Ed25519/RSA keys. Key generation and fingerprint viewing are deferred to Phase 3.

### Encryption at Rest

- **Host configs (SQLite):** Stored as plain JSON. No secrets in this layer.
- **Credentials:** Encrypted by the OS keystore at the platform level.
- **Session data:** Ephemeral — exists only in memory for the lifetime of a connection.

### Zero-Trust Sync (Phase 4)

Cross-device sync follows zero-trust principles:

1. **P2P transport** — Syncthing library or custom CRDT-based sync. No central server.
2. **TLS encryption** — All sync traffic encrypted in transit.
3. **End-to-end encryption** — Configs and keys encrypted with a user-controlled master key before leaving the device. Syncthing's built-in TLS provides transport security; application-layer encryption provides zero-trust.
4. **Conflict resolution** — CRDT-based (Conflict-free Replicated Data Type) for configs. Last-write-wins with user review for keys.
5. **No cloud** — No Sheil infrastructure. All sync is device-to-device.

---

## Agent Forwarding Design

`russh` supports the OpenSSH `auth-agent-req@openssh.com` channel type, enabling agent forwarding:

```
┌─────────────┐     SSH Connection       ┌─────────────┐
│   Client    │ ◄──────────────────────► │   Server    │
│   (Sheil)   │                          │  (Remote)   │
│             │                          │             │
│  ┌───────┐  │  auth-agent-req channel  │             │
│  │ Agent │◄─┼──────────────────────────┼─► SSH Agent │
│  │ (keys)│  │  forwarded agent traffic │  on server  │
│  └───────┘  │                          │             │
└─────────────┘                          └─────────────┘
```

**Flow:**

1. User imports keys into Sheil (stored in OS keystore via `tauri-plugin-keystore`).
2. On SSH connection, the agent forwarding flag is negotiated with the remote host.
3. When the remote host requests agent authentication (e.g., for a `git push` on the server), `russh` forwards the request to the Sheil agent process.
4. The agent retrieves the key from the keystore, performs the signing operation, and returns the response — all without the key leaving the keystore.

**Security considerations:**

- Agent forwarding is **opt-in per connection**. Users must explicitly enable it.
- The agent only responds to requests from authenticated SSH channels.
- Key material never leaves the OS keystore — the agent performs cryptographic operations via the keystore API.

---

## Cross-Platform Strategy

### Targets

| Platform | Architecture                            | Build Tool                                   | CI Runner                           |
| -------- | --------------------------------------- | -------------------------------------------- | ----------------------------------- |
| macOS    | aarch64 (Apple Silicon), x86_64 (Intel) | Tauri bundler (`tauri build`)                | macOS GitHub Actions runner         |
| Windows  | x86_64                                  | Tauri bundler → `.msi` / `.exe`              | Windows GitHub Actions runner       |
| Linux    | x86_64                                  | Tauri bundler → `.deb` / `.rpm` / AppImage   | Ubuntu GitHub Actions runner        |
| Android  | aarch64                                 | Tauri Android target (`tauri android build`) | macOS or Linux runner + Android SDK |
| iOS      | aarch64                                 | Tauri iOS target (`tauri ios build`)         | macOS runner + Xcode                |

### Platform-Specific Adaptations

**Desktop (macOS, Windows, Linux):**

- Full xterm.js with WebGL addon for GPU-accelerated rendering.
- Native keyboard shortcuts (Ctrl+C/V on Windows/Linux, Cmd+C/V on macOS).
- System tray / menu bar integration (Tauri `tray-icon`).

**Mobile (iOS, Android):**

- **Touch shim** — Custom touch event handler layer over xterm.js:
  - Single tap → move cursor
  - Double tap → select word
  - Long press → context menu (copy, paste, disconnect)
  - Two-finger pinch → zoom font size
  - Two-finger scroll → terminal scrollback
- **Column limit** — Default to 80 columns on mobile, max 120. xterm.js performance degrades at ≥200 cols on Android WebView.
- **Keyboard accessory bar** — Custom bar above the on-screen keyboard with: Tab, Esc, Ctrl, arrow keys, pipe (`|`), slash (`/`).
- **Background service** — `tauri-plugin-background-service` keeps SSH sessions alive:
  - Android: Foreground service with persistent notification.
  - iOS: BGTaskScheduler with limited background execution window.
- **Build pipeline** — Separate GitHub Actions workflows for Android (`.apk`) and iOS (requires Apple Developer account for signing).

### Build Configuration

Vite build targets are platform-aware (`vite.config.ts`):

- **Windows:** `chrome105` (WebView2)
- **Other platforms:** `safari13` (macOS WebKit, Linux WebKitGTK, iOS WKWebView, Android WebView)
- **Environment variables:** `TAURI_ENV_PLATFORM`, `TAURI_ENV_DEBUG` control build behavior.

### CSP and Security

Content Security Policy is currently `null` (permissive) in `tauri.conf.json` for development flexibility. Tightened CSP will be applied before stable release, restricting script sources and disabling `eval()`.

---

## Testing Strategy

### Frontend (Vitest + jsdom)

- **Unit tests** (`src/tests/unit/`) — Svelte component logic, utility functions, state management.
- **Integration tests** (`src/tests/integration/`) — Full component rendering with mocked Tauri IPC.
- **Tauri IPC mocking** — `src/tests/setup.ts` provides a `vi.fn()` stub for `@tauri-apps/api/core.invoke()`. Tests configure per-command responses via `mockInvoke.mockResolvedValueOnce()`.
- **Coverage:** All `$lib/*.ts` utilities and `$lib/components/**/*.svelte`.

### Backend (cargo test)

- **Unit tests** — Inline `#[cfg(test)] mod tests` in each Rust module.
- **I/O isolation** — `tempfile` crate for file-based command tests.
- **Error serialization tests** — Verify `{ name, message }` JSON shape for all error variants.

### CI

- `nub run test` runs: formatting check → linting → frontend unit tests → Rust tests.
- Pre-commit hook via Husky: `oxfmt` on all files, `nub run lint` on JS/TS.

---

## Project Structure

```
sheil/
├── src/                          Svelte 5 frontend
│   ├── main.ts                   Entry point — mount App
│   ├── app.css                   Tailwind + shadcn theme + dark mode
│   ├── App.svelte                Root component
│   ├── vite-env.d.ts             Vite type declarations
│   ├── lib/
│   │   ├── commands.svelte.ts    Tauri IPC wrappers + state classes
│   │   ├── utils.ts              cn(), type helpers
│   │   └── components/
│   │       ├── TerminalView.svelte    (planned)
│   │       ├── ConnectionManager.svelte (planned)
│   │       └── ui/               shadcn-svelte components
│   ├── tests/
│   │   ├── setup.ts              Test environment + Tauri IPC mock
│   │   ├── unit/                 Unit tests
│   │   │   ├── utils.test.ts
│   │   │   ├── runes/            Svelte 5 runes tests
│   │   │   └── ui/               Component unit tests
│   │   └── integration/          Integration tests
│   │       └── hello-world.test.ts
│   └── static/                   Static assets
│       └── favicon.png
├── tauri/                        Tauri 2 / Rust backend
│   ├── Cargo.toml                Rust dependencies
│   ├── build.rs                  Tauri build script
│   ├── tauri.conf.json           Tauri config (window, CSP, plugins, bundle)
│   ├── capabilities/
│   │   └── default.json          Permission grants
│   ├── src/
│   │   ├── main.rs               Windows subsystem + entry
│   │   ├── lib.rs                Tauri builder + plugin registration
│   │   └── commands/
│   │       ├── mod.rs
│   │       ├── default.rs        Read/write commands (placeholder)
│   │       └── errors.rs         Error types + serde
│   └── icons/                    App icons (all platforms)
├── docs/
│   └── ARCHITECTURE.md           This document
├── package.json                  Node dependencies + scripts
├── pnpm-lock.yaml                Dependency lockfile
├── svelte.config.js              Svelte preprocessor config
├── vite.config.ts                Vite + Svelte + Tailwind + test config
├── components.json               shadcn-svelte configuration
├── tsconfig.json                 TypeScript base config
├── tsconfig.app.json             Frontend TypeScript config
├── tsconfig.node.json            Node tooling TypeScript config
├── oxfmt.config.ts               oxfmt formatter config
├── oxlint.config.ts              oxlint linter config
├── index.html                    HTML entry point
├── CONTRIBUTING.md               Contribution guide (DCO, PR process)
├── LICENSE                       AGPLv3
└── README.md                     Project overview
```

---

## Key Design Decisions

### Why Tauri over Electron?

| Factor           | Electron                         | Tauri v2                                    |
| ---------------- | -------------------------------- | ------------------------------------------- |
| Bundle size      | 80–250MB                         | 600KB–10MB                                  |
| Memory (idle)    | 150–300MB                        | 30–40MB                                     |
| Backend language | Node.js                          | Rust                                        |
| Mobile support   | None (separate toolchain needed) | iOS + Android (built-in)                    |
| Plugin ecosystem | Vast (npm)                       | Growing (keystore, background-service, PTY) |

Tauri wins on bundle size, memory, and mobile support. The Rust backend also gives us direct access to `russh`, `serialport-rs`, and `sqlx` without Node.js native addon complexity.

### Why Svelte 5 over React or Vue?

| Factor         | React        | Vue          | Svelte 5                     |
| -------------- | ------------ | ------------ | ---------------------------- |
| Runtime size   | ~40KB        | ~33KB        | **0KB** (compiled)           |
| Virtual DOM    | Yes          | Yes          | **No**                       |
| Learning curve | Moderate     | Low          | Low                          |
| AI tooling     | Excellent    | Good         | Good                         |
| Reactive model | Hooks + deps | ref/reactive | **Runes** ($state, $derived) |

Svelte's zero-runtime approach means smaller bundles and no virtual DOM overhead — important for Tauri's already-small footprint. Runes provide a cleaner reactive model than hooks.

### Why russh over libssh2 or wrapping system SSH?

| Factor            | libssh2              | System OpenSSH    | russh                                |
| ----------------- | -------------------- | ----------------- | ------------------------------------ |
| Language          | C (FFI needed)       | Process spawn     | **Pure Rust**                        |
| Safety            | Unsafe FFI boundary  | String parsing    | Compile-time safety                  |
| SSH features      | Basic (no agent fwd) | Full              | **Full** (agent fwd, SFTP, port fwd) |
| Cross-compilation | Painful              | Depends on system | **Cargo cross**                      |
| License           | BSD                  | OpenSSH           | Apache 2.0                           |

`russh` gives us the full SSH feature set in pure Rust, with no unsafe FFI boundary or system dependency hell. The `w-ssh` reference project proves the russh + xterm.js integration works.

### Why SQLite over other embedded DBs?

- **Zero setup** — no server, no config. A single file.
- **sqlx** provides compile-time checked queries and async support.
- **Migrations** built-in via `sqlx::migrate!`.
- More than sufficient for host configs, tags, and snippets — this is not high-throughput OLTP.

---

## Future Considerations

### Mosh Protocol (Phase 4)

Mosh (Mobile Shell) uses UDP with the SSP (State Synchronization Protocol) on top. A Rust implementation would require:

- UDP socket management
- SSP state machine (synchronization, prediction, reconciliation)
- Integration with the terminal rendering pipeline (xterm.js receives synchronized state)

This is a significant engineering effort (4–6 weeks estimated). It is deferred to Phase 4.

### Plugin / Extension System (Phase 4)

A Rust trait-based plugin architecture loaded via `libloading` for community contributions. Plugins could extend:

- Connection protocols (new protocol handlers)
- Terminal addons (custom renderers, themes)
- Sync backends (custom sync providers)

### Web Version (Phase 4)

Tauri's web target combined with a WebSocket SSH proxy. The Rust backend runs as a server-side SSH proxy; the browser connects via WebSocket. The frontend (Svelte + xterm.js) runs unchanged in the browser.

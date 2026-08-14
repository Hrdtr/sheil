# Changelog

## v0.1.4 (2026-08-14)

### Features

- **terminal:** enable Unicode 11 support (062f888)
- **terminal:** bundle available terminal font options (29d1fc1)
- **terminal:** implement search functionality (05cd6db)
- **terminal:** add scroll sensitivity configuration (b2c0166)
- **terminal:** add minimum contrast ratio configuration (c40e809)
- **terminal:** add visual feedback for bell events (cc99fb6)
- **terminal:** add scroll-to-bottom button when scrolled up (84051bb)
- **terminal:** add terminal font size adjustment shortcuts (3c57b50)
- **terminal:** add keyboard shortcut to clear terminal (9e46403)
- **terminal:** add copy-on-select terminal behavior option (1ac735e)
- **terminal:** implement native-like clipboard handling (f12b9ea)

### Bug Fixes

- **ui:** ensure terminal dimensions are applied after channel opening (4f9d82a)

### Refactoring

- **ui:** overhaul settings interface (d063249)
- **terminal:** set default cursor inactive style to outline (b007c36)

### Documentation

- adjust tagline padding (76768bd)
- add product landing page and assets (8512969)

## v0.1.3 (2026-08-09)

### Bug Fixes

- **updater:** prevent reactive proxy from breaking update install (ac623b9)

## v0.1.2 (2026-08-08)

### Bug Fixes

- **ui:** update onboarding text and spacing in app root (9831d25)
- **ui:** enhance user onboarding and desktop usability (c8eafd6)

### Documentation

- update security policy supported versions (25580fb)
- restructure readme content for better clarity (db15110)

## v0.1.1 (2026-08-08)

### Features

- **updater:** implement in-app auto-updates (2502c0e)
- **release:** add automated release script and update CI workflows (b3b4cc6)

### Bug Fixes

- **ci:** add missing tauri signing key to ci build (ffee3af)
- **build:** set minimum macOS system version to 11.0 (b67b467)

### Refactoring

- **core:** refine session lifecycle and exit behavior (3f435bf)
- **ui:** improve layout styling and component structure (81e3760)

# TODO

## Next Up

- **Tray icon** — Use `tray-icon` crate (sister to `muda`, same raw window handle approach). Should show Bitwarden shield icon, right-click context menu with Lock/Quit. Iced PR https://github.com/iced-rs/iced/pull/3021 adds native tray support but is still **open** (targeting 1.0), so use `tray-icon` crate directly for now.
- **Executable icon** — Embed `.ico` in the Windows executable via `winresource` build script. Use the Bitwarden shield icon. Also set the window icon via iced's `window::Settings::icon`.
- **Sidebar expand/collapse animation** — Iced 0.14 has no built-in layout transitions. Would require a `Subscription` tick + interpolated width state (~50 lines). Not trivial but not huge.

## Developer Experience

- **Hot reloading** — Iced PR https://github.com/iced-rs/iced/pull/3000 is **merged** into master (June 2025). Uses `hot` feature flag + `subsecond`/`cargo-hot`. Not in iced 0.14 release yet — requires iced from git or waiting for 0.15/1.0. Worth switching to when available.

## UI Polish

- Sidebar: indented tree hierarchy (Vault > All vaults > My vault)
- TOTP circular timer in detail pane (currently placeholder text)
- Account switcher dropdown: visual update to match 2025 Figma (Lock/Logout buttons, Options section)
- Wire native menu items to actual app actions (lock, quit, etc.)
- Light mode / theme system (refactor `theme.rs` from constants to a Theme struct)
- SVG logo antialiasing — Iced's resvg rasterizer doesn't match browser quality; consider pre-rasterized PNG or splitting into SVG shield + text widget

## Refactoring

- **Theme system** — Refactor `theme.rs` from module-level `const` colors to a `Theme` struct with variants (Dark, Light). All widgets should pull colors from the active theme rather than importing constants directly. This is a prerequisite for light mode.
- **Color tokens** — Define semantic color roles (e.g. `bg_primary`, `bg_surface`, `fg_brand`, `border_default`) rather than using raw hex everywhere. This makes theme switching and consistency easier. Note: several constants already share values (HEADER_BG == BORDER, CARD_BG == SIDEBAR_SELECTED) — semantic tokens would make this clearer.

## SDK Preparation

- **Structured mock layer** — Replace flat `mock.rs` with a mock that mirrors the real SDK's `PasswordManagerClient` structure (e.g. `client.vault().ciphers().list()`, `client.unlock().unlock_with_password()`). This would make the eventual SDK swap a thin adapter change rather than a rewrite of the data flow. The mock should define traits/interfaces matching the SDK surface area so views and state don't depend on concrete types.

## Functionality

- Wire copy buttons in detail pane to clipboard (arboard crate or iced clipboard API)
- Wire edit/delete buttons in detail pane (currently stubs)
- Wire menu "Quit" to actually close the app
- Wire menu "Lock" / "Lock All" to lock user sessions
- Handle tray icon click to show/hide window
- "Unlock with Windows Hello" button (future, needs keyring/biometric integration)

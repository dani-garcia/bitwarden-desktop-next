# TODO

## Next Up

- **Tray icon** — Use `tray-icon` crate (sister to `muda`, same raw window handle approach). Should show Bitwarden shield icon, right-click context menu with Lock/Quit. Iced PR https://github.com/iced-rs/iced/pull/3021 adds native tray support but is still **open** (targeting 1.0), so use `tray-icon` crate directly for now.
- **Avatar color auto-generation** — Generate avatar background color from username/email hash (like the official app does) instead of using a fixed color.

## UI Polish

- Sidebar: indented tree hierarchy (Vault > All vaults > My vault)
- Sidebar expand/collapse animation (Iced 0.14 has no built-in layout transitions; needs `Subscription` tick + interpolated width)
- TOTP circular timer in detail pane (currently placeholder text)
- Account switcher dropdown: visual update to match 2025 Figma (Lock/Logout buttons, Options section)
- SVG logo antialiasing — Iced's resvg rasterizer doesn't match browser quality; consider pre-rasterized PNG

## Auth Flow

- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields.
- **Master password hint request** — "Get master password hint" link on login password screen. Sends hint request to server.
- **Self-hosted server URL modal** — Server selector "Self-hosted" option should open a modal to input custom server URL.
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

## Functionality

- Wire copy buttons in detail pane to clipboard (arboard crate or iced clipboard API)
- Wire edit/delete buttons in detail pane (currently stubs)
- Handle tray icon click to show/hide window
- "Unlock with Windows Hello" button (future, needs keyring/biometric integration)

## Testing

- **Unit tests for pure logic** — `VaultView::filtered_items()`, `Shortcut::matches()`, `EnabledWhen::check()`, view `update()` state machines (message in → actions out). Standard `#[test]`, no framework needed.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test = "0.14"` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

## Developer Experience

- **Hot reloading** — Iced PR https://github.com/iced-rs/iced/pull/3000 is **merged** into master (June 2025). Uses `hot` feature flag + `subsecond`/`cargo-hot`. Not in iced 0.14 release yet — requires iced from git or waiting for 0.15/1.0.

## SDK Integration

- **Fill mock clients with fake data** — `sdk.rs` has `mock_personal_client()` and `mock_work_client()` with empty `MemoryRepo`s. Populate them with realistic `Cipher` and `Folder` data (matching what `mock.rs` currently provides) so the vault view can read from the SDK instead of flat structs.
- **Wire SDK to application logic** — Replace `mock::mock_users()` and the flat `UserSession.vault_items` with data sourced from `ClientManager`. The vault view should read cipher/folder data through `PasswordManagerClient.vault()` rather than the current `CipherItem` structs. This involves updating `App`, `refresh_cache()`, and the vault view to use SDK types.

## Toast Notifications

- **Research existing toast/notification widgets** — Before building a custom implementation, evaluate:
  - Iced's own toast example: https://github.com/iced-rs/iced/blob/master/examples/toast/src/main.rs
  - `iced-toasts` crate: https://github.com/Gomango999/iced-toasts/tree/main
  - Check if `iced_aw` has any notification/toast widget
  - Determine which approach fits best (overlay-based, stacked, timed auto-dismiss, action buttons)
- **Implement toast system** — Needed for user feedback on actions like copy-to-clipboard, unlock success/failure, network errors, sync status. Should support multiple concurrent toasts, auto-dismiss with timeout, and different severity levels (info, success, error).

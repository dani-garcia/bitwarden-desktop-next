# Decisions

## Framework: Iced 0.14

**Decision**: Use Iced as the GUI framework.

**Alternatives considered**: Dioxus (webview-based desktop mode, custom renderer still maturing), Slint (GPL/commercial license concern).

**Rationale**: Pure Rust, wgpu rendering, no webview, Elm architecture ensures repaints only on state change, ~25k GitHub stars, actively maintained, cross-platform.

## Crate Name: bitwarden-desktop-native

**Decision**: Named `bitwarden-desktop-native` to avoid conflict with existing "bitwarden-lite" project.

## UI Only (No Business Logic)

**Decision**: This app is stub UI only. All crypto, API, authentication, and vault operations will come from a separate SDK (`PasswordManagerClient`).

**Rationale**: Separation of concerns. The SDK handles security-sensitive operations; this project handles presentation.

## Multi-User State Model

**Decision**: App state uses `HashMap<UserId, UserSession>` mirroring the SDK's `HashMap<UserId, PasswordManagerClient>`.

**Rationale**: Makes future SDK integration a clean swap. Multiple accounts can be available simultaneously with one active at a time.

## DEV_SCREEN Environment Variable

**Decision**: Use `DEV_SCREEN` env var (not compile-time feature flags) to skip to specific screens during development.

**Rationale**: Avoids recompilation when screenshotting different views. `DEV_SCREEN=vault cargo run` skips directly to vault.

## Colors: Picker Over SCSS

**Decision**: Use color-picked values from the actual running Bitwarden app, not SCSS/CSS variable values.

**Rationale**: The SCSS values in `variables.scss` and Tailwind CSS vars don't match what the app actually renders. For example, `backgroundColor` is `#1f242e` in SCSS but the actual window background is `#070b18`. Always verify with a color picker (Win+Shift+C with PowerToys).

## Native Menus: muda

**Decision**: Use `muda` crate for native OS menus rather than drawing menus inside iced.

**Alternatives considered**: `iced_aw` menu widget (draws inside iced window, same on all platforms), custom drawn menu bar.

**Rationale**: Native menus give correct platform behavior automatically — Windows gets the classic menu bar under the title bar, macOS gets the system menu bar at the top of the screen. The `muda` crate is from the same ecosystem as `tray-icon` and works with raw window handles from winit (which iced uses internally).

**Implementation**: Menu is attached via iced subscription that listens for `Window::Opened` event, then queries the raw window handle via `window::raw_id()`, and calls `menu.init_for_hwnd()`.

## Font: Inter

**Decision**: Bundle the Inter variable font (`InterVariable.ttf`) and set it as the iced default font.

**Rationale**: The official Bitwarden app uses Inter (`$font-family-sans-serif: Inter` in `variables.scss`). Inter is heavier than the default system font, which gives the UI the same visual weight as the original.

## Button Style: Pill-Shaped (20px radius)

**Decision**: Use 20px border-radius for buttons (more pill-shaped than the 12px from the component library).

**Rationale**: Visual inspection of the actual rendered app shows more rounded buttons than the Tailwind `tw-rounded-xl` (12px) suggests. 20px matches the actual look better.

## Theme System (TODO)

**Decision**: Refactor from module-level color constants (`theme.rs`) to a trait-based or struct-based theme system.

**Status**: Not yet implemented. Currently all colors are `const` in `theme.rs`.

**Rationale**: The app needs to support light/dark mode and potentially custom themes. A `Theme` struct with variants would allow runtime switching.

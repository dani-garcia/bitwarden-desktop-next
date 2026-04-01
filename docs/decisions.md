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

## Button Style: New Component Library (12px radius)

**Decision**: Match the newer Tailwind-based component library styles, not the legacy SCSS.

**Rationale**: The official app is migrating to the new component library (`clients/libs/components/`). The lock screen in current Bitwarden desktop uses the newer rounded buttons (12px radius), not the legacy 3px.

## Theme System (TODO)

**Decision**: Refactor from module-level color constants (`theme.rs`) to a trait-based or struct-based theme system.

**Status**: Not yet implemented. Currently all colors are `const` in `theme.rs`.

**Rationale**: The app needs to support light/dark mode and potentially custom themes. A `Theme` struct with variants would allow runtime switching.

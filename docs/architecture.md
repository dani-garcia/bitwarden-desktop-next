# Architecture

## Overview

**bitwarden-desktop-native** is a lightweight Rust alternative to the official Bitwarden desktop app (Electron/Angular). It uses the **Iced 0.14** GUI framework with an Elm-style architecture.

## Tech Stack

- **Language**: Rust (2024 edition)
- **GUI**: Iced 0.14 (wgpu/tiny-skia rendering, no webview)
- **Architecture**: Elm pattern — `Message` enums drive state transitions via `update()`, `view()` renders UI from state

## Project Structure

```
src/
├── main.rs              # Entry point, window config (900x600, min 600x480)
├── app.rs               # Root Application: state, message routing, view dispatch
├── theme.rs             # Color constants (currently module-level consts, needs refactor to trait-based)
├── state.rs             # AppState, UserSession, CipherItem, Screen, SidebarFilter enums
├── mock.rs              # Mock data: 2 fake users with vault items
├── views/
│   ├── mod.rs
│   ├── login.rs         # Lock/unlock screen
│   └── vault.rs         # Main vault view (top bar + sidebar + item list)
└── widgets/
    ├── mod.rs
    ├── sidebar.rs       # Category tree navigation
    ├── item_list.rs     # Scrollable vault items with colored initial circles
    ├── search_bar.rs    # Search input with filtering
    └── account_switcher.rs  # Multi-account dropdown
```

## State Model

```rust
struct AppState {
    users: HashMap<UserId, UserSession>,  // Mirrors future SDK HashMap<UserId, PasswordManagerClient>
    active_user: Option<UserId>,
    screen: Screen,  // Login | Vault
}
```

The app holds multiple user sessions simultaneously. Switching to a locked user navigates to the unlock screen; switching to an unlocked user shows their vault.

## Future SDK Integration

The business logic will come from a separate Rust SDK centered around `PasswordManagerClient`:

```rust
// Login
let client = LoginClient::new().login_with_password(params).await?;
// Restore from state
let client = PasswordManagerClient::load_from_state(params).await?;
client.unlock().unlock_with_password(pw).await?;
let ciphers = client.vault().ciphers().list().await?;
```

The current stub uses mock data in place of this SDK. The state model is designed so swapping in the real SDK is straightforward.

## Dev Shortcuts

- `DEV_SCREEN=vault cargo run` — Skip login, start directly on vault screen (no recompilation needed)
- `screenshot.ps1` — PowerShell script to capture the app window by title ("Bitwarden")

## Key Dependencies

- `iced = "0.14"` with `tokio` feature — the only runtime dependency

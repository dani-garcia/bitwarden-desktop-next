# Architecture

## Overview

**bitwarden-desktop-native** is a lightweight Rust alternative to the official Bitwarden desktop app (Electron/Angular). It uses the **Iced 0.14** GUI framework with an Elm-style architecture.

## Tech Stack

- **Language**: Rust (2024 edition)
- **GUI**: Iced 0.14 (wgpu/tiny-skia rendering, no webview)
- **Native menus**: muda (cross-platform, uses HWND on Windows, NSApp on macOS)
- **Font**: Inter (variable, bundled in `assets/InterVariable.ttf`)
- **Architecture**: Elm pattern — `Message` enums drive state transitions via `update()`, `view()` renders UI from state

## Project Structure

```
src/
├── main.rs              # Entry point, window config (1080x720, min 600x480), font loading
├── app.rs               # Root Application: state, message routing, view dispatch, subscriptions
├── menu.rs              # Native menu bar (muda): build + attach via raw window handle
├── theme.rs             # Color constants (TODO: refactor to trait-based theme for light/dark)
├── state.rs             # AppState, UserSession, CipherItem, Screen, SidebarFilter
├── mock.rs              # Mock data: 2 fake users with vault items and server URLs
├── views/
│   ├── mod.rs
│   ├── login.rs         # Lock/unlock screen (logo, lock icon, card, bg illustrations, status)
│   └── vault.rs         # Main vault view (top bar + sidebar + item list)
└── widgets/
    ├── mod.rs
    ├── sidebar.rs       # Category tree navigation with bitwarden header
    ├── item_list.rs     # Scrollable vault items with colored initial circles
    ├── search_bar.rs    # Search input with filtering
    └── account_switcher.rs  # Trigger (email + server + arrow) + floating dropdown

assets/
├── InterVariable.ttf    # Inter font (matches official app)
├── logo-white.svg       # Bitwarden wordmark + shield icon
├── lock-icon.svg        # Lock screen icon (shield with stars)
├── bg-left.svg          # Left background illustration (login screen)
└── bg-right.svg         # Right background illustration (login screen)
```

## State Model

```rust
struct AppState {
    users: HashMap<UserId, UserSession>,  // Mirrors future SDK HashMap<UserId, PasswordManagerClient>
    active_user: Option<UserId>,
    screen: Screen,  // Login | Vault
}

struct UserSession {
    email: String,
    display_name: String,
    server_url: String,      // e.g. "bitwarden.com"
    locked: bool,
    vault_items: Vec<CipherItem>,
}
```

The app holds multiple user sessions simultaneously. Switching to a locked user navigates to the unlock screen; switching to an unlocked user shows their vault.

## Message Flow

```
App::new() → (State, Task::none())
  ↓
Subscription: listen for Window::Opened event
  ↓
WindowOpened(id) → raw_id(id) Task
  ↓
GotRawId(hwnd) → menu::attach_menu(hwnd) (native menu bar appears)
  ↓
User interaction → Login/Vault messages → update() → view()
```

## Native Menu Integration

The `muda` crate provides cross-platform native menus:
- On Windows: attaches to the HWND via `init_for_hwnd()`
- On macOS: attaches to the NSApp via `init_for_nsapp()`

The raw window handle is obtained through iced's `window::raw_id()` Task, triggered by a subscription that listens for the `Window::Opened` event. The menu is built and leaked (`std::mem::forget`) so it lives for the process lifetime.

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

- `DEV_SCREEN=vault cargo run` — Skip login, start directly on vault screen
- `screenshot.ps1` — Capture app window by title
- `screenshot-all.ps1` — Automated login + vault screenshots to a versioned folder

## Key Dependencies

- `iced = "0.14"` with `tokio` and `svg` features — GUI framework
- `muda = "0.15"` — Native OS menus (File, Edit, View, Account, Window, Help)

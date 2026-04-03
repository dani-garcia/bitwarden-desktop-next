# bitwarden-desktop-native

Lightweight Rust alternative to the Bitwarden desktop app using Iced 0.14 GUI framework.

## Quick Start

```bash
cargo run                      # Starts on login screen
cargo clippy                   # Lint check (must pass clean)
```

### Dev Modes

```bash
DEV_BOTH_MENUS=1 cargo run     # Show native + custom menus side-by-side
```

## Project Context

- **UI-only stub** — no business logic, crypto, or API calls. A separate SDK (`PasswordManagerClient`) will handle that later.
- **Multi-user** — state holds `HashMap<UserId, UserSession>`, mirrors the future SDK model.
- **Iced 0.14** — Elm architecture (message-driven updates). Uses `iced` with `tokio`, `svg`, `advanced` features. `button::Style` requires a `snap: false` field.
- **Custom theme type** — `AppTheme` implements `iced::theme::Base` + widget `Catalog` traits. Dark/light themes with runtime switching (Help > About Bitwarden). All `.style()` closures receive `&AppTheme`.
- **Dual menu system** — Custom-drawn title bar on Windows/Linux, native muda menu on macOS. Both driven by a single `MENUS` definition with shortcuts, enabled states, and actions. `DEV_BOTH_MENUS=1` shows both simultaneously.

## Project Structure

```
src/
  app.rs                        — Root App (11 fields), thin message dispatcher, view structs
  main.rs                       — Entry point, window config, font loading
  menu.rs                       — MENUS definition, Shortcut, MenuAction, MenuState, native muda bridge
  mock.rs                       — Fake users/vault items (each item has a stable `id` field)
  state.rs                      — Core types: AppState, CipherItem, Screen, etc.

  theme/
    mod.rs                      — AppTheme, AppColors, Base impl, radius constants
    dark.rs                     — Dark palette (colors from actual Bitwarden app)
    light.rs                    — Light palette (placeholder)
    catalog.rs                  — Widget Catalog trait impls for AppTheme

  components/
    mod.rs                      — separator_h(), separator_v(), styled_card() helpers
    buttons.rs                  — primary(), secondary(), ghost(), ghost_icon(), transparent()
    icons.rs                    — Bootstrap Icons + BWI icons with .render() and .input_icon()
    account_switcher.rs         — Shared account dropdown (used by login + vault)
    drop_down.rs                — Local fork of iced_aw DropDown with BelowLeft/BelowRight/AboveRight

  views/
    login/
      mod.rs                    — LoginView struct + LoginAction + view()
    vault/
      mod.rs                    — VaultView struct + VaultAction + PaneKind + view()
      widgets/
        sidebar.rs              — Icon rail + expanded nav panel
        item_list.rs            — Vault item table with action icons
        detail_pane.rs          — Item detail view (readonly fields, cards)
        search_bar.rs           — Search input with native text_input icon
    title_bar/
      mod.rs                    — TitleBarState + TitleBarAction + view()
      window_chrome.rs          — Platform chrome icons, resize wrapper
      dropdown.rs               — Menu dropdown panels + submenu rendering
```

## Why Things Are the Way They Are

- **Custom `AppTheme` instead of `iced::Theme`**: iced 0.14's `application()` is generic over Theme. Our type gives `&AppTheme` in ALL `.style()` closures, with direct access to 20+ semantic color tokens. No thread_local, no registry, no downcasting.
- **Local `DropDown` fork instead of `iced_aw`**: We needed `BelowLeft`/`BelowRight` alignments that upstream doesn't have. The fork is ~500 lines, identical logic except the added alignment variants.
- **Custom title bar on Windows instead of native menus**: Native Win32 menus via `muda::init_for_hwnd()` create a 1px transparent gap (DWM compositor border). Drawing ourselves eliminates it.
- **`refresh_cache()` still exists**: `view()` returns `Element<'_, ...>` borrowing from `&self`. Locally computed `Vec`s in `view()` would be dropped before the Element. Cached fields on the struct survive the borrow. This is an iced lifetime constraint, not a design choice.
- **View structs return `Vec<Action>` not `Task`**: Views don't know about window IDs, global state, or other views. They express intent ("switch user", "focus search") and App translates to concrete operations.
- **`NativeMenuHandle` on App, not static**: Avoids global mutable state. Supports potential multi-window future.

## Architecture: State Decentralization

Each view owns its state and handles its own messages. Cross-cutting effects propagate upward via action enums:

- **`LoginView`** — owns `password_input`, `show_password`, `dropdown_open`. Returns `Vec<LoginAction>` (Unlock, LogOut, SwitchUser).
- **`VaultView`** — owns search, filter, selection, sidebar, pane state, item cache. Returns `Vec<VaultAction>` (SwitchUser, FocusSearch).
- **`TitleBarState`** — owns `open_menu`, `open_submenu`. Returns `Vec<TitleBarAction>` (MenuAction, window ops).
- **`App`** (11 fields) — thin dispatcher. Delegates to views, processes returned actions, handles cross-cutting concerns (screen switch, user switch, menu actions).

### Message Flow Example

```
User clicks "Unlock" button
  → login view emits LoginMessage::Unlock
  → iced delivers Message::Login(LoginMessage::Unlock) to App::update()
  → App calls self.login_view.update(msg)
  → LoginView clears password_input, returns vec![LoginAction::Unlock]
  → App processes action: unlocks active session, sets Screen::Vault
  → App calls self.refresh_cache() (recomputes email, accounts, vault items)
  → iced calls App::view() → renders vault screen
```

### Cross-Cutting Dismissal

When any view message arrives, App also dismisses the other view's overlays:
- `Message::Login(_)` / `Message::Vault(_)` → `self.title_bar.dismiss_menu()`
- `Message::TitleBar(_)` → closes active view's `dropdown_open`

## Coding Conventions

### Imports
- Group imports: `use crate::{a, b};` not separate `use crate::a; use crate::b;`
- Nest component imports: `use crate::components::{buttons, icons, account_switcher};`
- Import commonly-used iced types directly: `Background`, `Border`, `Color`, `Shadow`, `Alignment`.

### Padding
- Use `[v, h]` shorthand: `.padding([8, 16])`.
- Use `Padding { top, right, bottom, left }` only for asymmetric cases.

### Theming
- All colors come from `AppTheme.colors`. Never hardcode outside `theme/dark.rs` and `theme/light.rs`.
- Radii (`RADIUS_SM/MD/LG/PILL`) are structural constants, not theme-dependent.
- In `.style()` closures: `theme.colors.xxx`. For `text().color()` / `icon.render()`: pass `&AppColors`.

### Button Components
- Use `components::buttons::{primary, secondary, ghost, ghost_icon, transparent}(content)`.
- Takes `impl Into<Element>`, returns `Button` for chaining.

### Icons
- `icon.render(size, color)` — renders as Element for general use.
- `icon.input_icon(size, side)` — builds `text_input::Icon` for use with `TextInput::icon()`.
- `icon.char()` is `pub(crate)` — prefer `render()` or `input_icon()`.

### Dropdowns
- Use `components::drop_down::DropDown` with custom alignments: `BelowLeft`, `BelowRight`, `AboveRight`.
- Always set `.on_dismiss(message)` for click-outside-to-close.
- Cross-message dismissal: `Message::TitleBar` closes account dropdown, `Message::Login`/`Vault` closes title bar menu.

### Dead Code
- Use `#[expect(dead_code)]` (not `#[allow]`) — warns if suppression becomes unnecessary.

### Menu System
- `MENUS` static drives both custom title bar rendering and native muda menus.
- `Shortcut::to_accelerator()` converts to muda's `Accelerator` via string parsing.
- `NativeMenuHandle` on App stores action map + item handles (no static mutable state).
- `poll_native_event()` polled via 16ms iced subscription, dispatches to `handle_menu_action()`.
- `sync_native_enabled()` called from `refresh_cache()` to keep native items in sync.

## Reference App

The official Bitwarden app is in `clients/` (git submodule). Key locations:
- Button styles: `clients/libs/components/src/button/button.component.ts`
- Logo SVG: `clients/libs/assets/src/svg/svgs/password-manager.ts`
- Background illustrations: `clients/libs/assets/src/svg/svgs/background-{left,right}-illustration.ts`
- BWI icon codepoints: `clients/libs/angular/src/scss/bwicons/styles/style.scss`
- Menu entries: `clients/apps/desktop/src/main/menu/menu.*.ts`

## Docs (IMPORTANT: read these before making changes)

- `docs/architecture.md` — Current structure, state model, menu system, icon system
- `docs/decisions.md` — Framework choice, theme system, overlay approach, state decentralization, menu unification
- `docs/todo.md` — Pending work items, next up tasks
- `docs/design-reference.md` — Official app findings: button styles, colors, logos, illustrations

## Design Notes

- All colors are theme-swappable. Dark/light toggle via Help > About Bitwarden.
- Account switcher uses DropDown overlay with `BelowRight` alignment.
- Sidebar uses BWI icons (not Bootstrap Icons) to match the official app.
- SVG logo antialiasing: Iced's resvg rasterizer doesn't match browser-quality. Container constrains visual size while SVG fills available width.

## Iced Gotchas

- `button::Style` requires `snap: false` — missing it causes a compile error with no obvious message.
- `view()` returns `Element<'_, M, Theme>` borrowing from `&self`. Locally computed `Vec`s can't be borrowed into the returned Element — use cached fields on the struct instead.
- iced overlays only support ONE level — a `DropDown` inside another `DropDown`'s overlay won't render its own overlay. Submenus must be part of the same overlay content (e.g. a `row![main_panel, submenu]`).
- `iced::time::every()` is useful for polling external event sources (like muda's `MenuEvent::receiver()`).
- `widget::operation::focus(Id)` is needed after PaneGrid rebuilds to keep text input focus.
- The `iced_aw` crate (0.13) is kept as a dependency for source reference but `default-features = false` — we use our own `drop_down.rs` fork.

## Source Reference Locations

When investigating iced internals, the cargo registry cache has the source:
- iced core: `~/.cargo/registry/src/*/iced_core-0.14.0/src/`
- iced widgets: `~/.cargo/registry/src/*/iced_widget-0.14.2/src/`
- iced_aw: `~/.cargo/registry/src/*/iced_aw-0.13.1/src/`
- muda: `~/.cargo/registry/src/*/muda-0.17.1/src/`

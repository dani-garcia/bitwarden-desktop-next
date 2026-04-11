# bitwarden-desktop-next

Lightweight Rust alternative to the Bitwarden desktop app using Iced 0.14 GUI framework.

## Quick Start

```bash
cargo run                      # Starts on login screen
cargo clippy                   # Lint check (must pass clean)
cargo run --bin packager       # Package .app/.dmg/.msi via cargo-packager
```

### Dev Modes

```bash
DEV_BOTH_MENUS=1 cargo run     # Show native + custom menus side-by-side
```

## Project Context

- **Cargo workspace** — root `Cargo.toml` defines members: `crates/desktop` (default), `bitwarden_license/*`, `tools/*`. Desktop crate is the only default member.
- **UI-only stub** — no business logic, crypto, or API calls. A separate SDK (`PasswordManagerClient`) will handle that later.
- **Multi-user** — state holds `HashMap<UserId, UserSession>`, mirrors the future SDK model.
- **Iced 0.14** — Elm architecture (message-driven updates). Uses `iced` with `tokio`, `svg`, `advanced` features. `button::Style` requires a `snap: false` field.
- **Custom theme type** — `AppTheme` implements `iced::theme::Base` + widget `Catalog` traits. Light theme is the default; dark/light toggle via Help > About Bitwarden. All `.style()` closures receive `&AppTheme`.
- **Dual menu system** — Custom-drawn title bar on Windows/Linux, native muda menu on macOS. Both driven by a single `MENUS` definition with shortcuts, enabled states, and actions. `DEV_BOTH_MENUS=1` shows both simultaneously.

## Project Structure

```
Cargo.toml                      — Workspace root (members: crates/desktop, bitwarden_license/*, tools/*)
Packager.toml                   — cargo-packager config (.app, .dmg, .msi, signing)
LICENSE.txt / LICENSE_*.txt     — License files

crates/desktop/                 — Main desktop application crate
  src/
    app.rs                      — Root App (11 fields), thin message dispatcher, view structs
    main.rs                     — Entry point, window config, font loading, APP_FONT/APP_FONT_BOLD constants
    menu.rs                     — MENUS definition, Shortcut, MenuAction, MenuState, native muda bridge
    mock.rs                     — Fake users/vault items (each item has a stable `id` field)
    state.rs                    — Core types: AppState, CipherItem, Screen, etc.

    theme/
      mod.rs                    — AppTheme, AppColors, Base impl, radius constants
      dark.rs                   — Dark palette (colors from actual Bitwarden app)
      light.rs                  — Light palette (colors from design mockup)
      catalog.rs                — Widget Catalog trait impls for AppTheme

    components/
      mod.rs                    — separator_h(), separator_v(), styled_card() helpers
      buttons.rs                — primary(), secondary(), ghost(), ghost_icon(), transparent()
      icons.rs                  — Bootstrap Icons + BWI icons with .render() and .input_icon()
      account_switcher.rs       — Shared account dropdown (used by login + vault)
      drop_down.rs              — Local fork of iced_aw DropDown with BelowLeft/BelowRight/AboveRight

    views/
      login/
        mod.rs                  — LoginView struct + LoginEvent + view()
      vault/
        mod.rs                  — VaultView struct + VaultEvent + PaneKind + view()
        widgets/
          sidebar.rs            — Icon rail + expanded nav panel
          item_list.rs          — Vault item table with action icons
          detail_pane.rs        — Item detail view (readonly fields, cards)
          search_bar.rs         — Search input with native text_input icon
      title_bar/
        mod.rs                  — TitleBarState + TitleBarEvent + WindowCommand + view() + view_empty()
        window_chrome.rs        — Platform chrome icons, resize wrapper
        dropdown.rs             — Menu dropdown panels + submenu rendering

tools/
  packager/                     — Wrapper crate that invokes cargo_packager::cli::run

bitwarden_license/              — Licensed workspace members (future)
```

## Why Things Are the Way They Are

- **Custom `AppTheme` instead of `iced::Theme`**: iced 0.14's `application()` is generic over Theme. Our type gives `&AppTheme` in ALL `.style()` closures, with direct access to 20+ semantic color tokens. No thread_local, no registry, no downcasting.
- **Local `DropDown` fork instead of `iced_aw`**: We needed `BelowLeft`/`BelowRight` alignments that upstream doesn't have. The fork is ~500 lines, identical logic except the added alignment variants.
- **Custom title bar on Windows instead of native menus**: Native Win32 menus via `muda::init_for_hwnd()` create a 1px transparent gap (DWM compositor border). Drawing ourselves eliminates it.
- **`refresh_cache()` still exists**: `view()` returns `Element<'_, ...>` borrowing from `&self`. Locally computed `Vec`s in `view()` would be dropped before the Element. Cached fields on the struct survive the borrow. This is an iced lifetime constraint, not a design choice.
- **Sub-views return `(Task<SubMessage>, Option<Event>)`**: copies Halloy's compositional MVU pattern. Sub-views own their async lifecycle (they call `Task::perform` directly, with `&Arc<ClientManager>` injected at call time) so `app.rs` stops being a shared-write bottleneck when new async operations are added. Events are declarative domain facts ("Unlocked", "UserSelected") that App routes into side effects. See [docs/decisions.md](docs/decisions.md) → "View Architecture: Compositional MVU" for the full rationale.
- **`NativeMenuHandle` on App, not static**: Avoids global mutable state. Supports potential multi-window future.

## Architecture: Compositional MVU

Each sub-view owns its local state AND its async work, exposing:

```rust
pub fn update(
    &mut self,
    msg: SubMessage,
    client_manager: &Arc<ClientManager>,
    active_user: Option<&UserId>,
) -> (Task<SubMessage>, Option<SubEvent>);
```

- **`LoginView`** — owns `password_input`, `show_password`, `dropdown_open`, `auth_page`. Runs `Task::perform(mgr.unlock(...))` directly; emits `LoginEvent::Unlocked { uid }` on completion. Events: `Unlocked` / `LoggedIn` / `SignOutRequested` / `UserSelected` / `AddAccountRequested` / `ToastRequested`.
- **`VaultView`** — owns search, filter, selection, sidebar, pane state, item cache. Runs `Task::perform(mgr.list_ciphers(...))` and `full_cipher(...)` directly; handles `VaultMessage::ListLoaded` / `DetailLoaded` internally with stale-checks. Events: `UserSelected` / `AddAccountRequested` / `SearchFocusRequested` / `ToastRequested`.
- **`TitleBarState`** — owns `open_menu`, `open_submenu`. No async work. Events: `MenuInvoked(MenuAction)` / `Window(WindowCommand)`.
- **`App`** — thin router. Dispatches messages to sub-views via the compositional signature, lifts returned tasks with `.map(Message::Sub)`, translates events into side effects via `handle_*_event` methods.

The top-level `Message` enum is 5 variants: `Login(LoginMessage)`, `Vault(VaultMessage)`, `TitleBar(TitleBarMessage)`, `Window(WindowMessage)` (per-window OS events, daemon-ready), `System(SystemMessage)` (global signals). **No orphan async callback variants** — they live inside sub-enums (e.g. `LoginMessage::UnlockCompleted`, `VaultMessage::ListLoaded`).

### Message Flow Example

```
User clicks "Unlock" button
  → login view emits LoginMessage::Unlock
  → iced delivers Message::Login(LoginMessage::Unlock) to App::update()
  → App router calls self.login_view.update(msg, &client_manager, active_user)
  → LoginView drains password, runs Task::perform(mgr.unlock(uid, pw))
     and returns (Task<LoginMessage>, None)
  → App lifts the task with .map(Message::Login) and runs it
  → task resolves; Message::Login(LoginMessage::UnlockCompleted(uid, Ok(()))) re-enters
  → App router calls self.login_view.update(completion, ...)
  → LoginView stale-checks active_user, returns (Task::none(), Some(LoginEvent::Unlocked { uid }))
  → App's handle_login_event flips Screen::Vault and returns load_vault_list_task
  → task resolves; Message::Vault(VaultMessage::ListLoaded(uid, Ok(items))) re-enters
  → VaultView handles it internally (stale-check, self.set_items(items))
  → App calls post_update() → refresh_cache() → iced calls App::view() → vault screen renders
```

Every async step re-enters through the owning view, keeping the async lifecycle local.

### Cross-Cutting Dismissal

LOAD-BEARING: the first `match &message` block at the top of `App::update` dismisses the other view's overlays on every sub-view message:
- `Message::Login(_)` / `Message::Vault(_)` → `self.title_bar.dismiss_menu()`
- `Message::TitleBar(_)` → `self.login_view.dismiss_dropdowns()` + `self.vault_view.dismiss_dropdowns()`

Without this block, a menu click from the vault would leave the account-switcher dropdown hanging. Sub-views provide `dismiss_dropdowns()` helpers; the policy itself stays at the router level so sub-views don't need to know about each other.

### How to add a new view

See [docs/architecture.md](docs/architecture.md) → "How to Add a New View" for the canonical 7-step recipe. Summary: create `views/<name>/mod.rs` with a `<Name>View` struct, `<Name>Message` enum, `<Name>Event` enum, and the `(Task, Option<Event>)` update signature; then in `app.rs` add a `Message::<Name>` variant, a view field on `App`, a router arm, and a `handle_<name>_event` method. Four mechanical adds in `app.rs`; everything else stays inside the view directory. That's the full multi-team story.

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
- Sidebar-specific tokens: `nav_text` (white in both themes since sidebar is always dark blue) and `nav_item_hover` for sidebar hover state.
- Font sizes are consolidated to 5 values: 12, 14, 16, 18, 28.

### Button Components
- Use `components::buttons::{primary, secondary, ghost, ghost_icon, transparent}(content)`.
- Takes `impl Into<Element>`, returns `Button` for chaining.
- `ghost(content, hover_bg)` and `ghost_icon(content, hover_bg)` take an explicit `hover_bg: Color` parameter instead of reading from theme (needed because sidebar has different bg than content area).

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

## Font

- **Inter 18pt** — static weight files: `Inter_18pt-Medium.ttf` (default) + `Inter_18pt-Bold.ttf`. Family name: `"Inter 18pt"`.
- `APP_FONT` and `APP_FONT_BOLD` constants in `main.rs` reference these fonts.
- Default weight is Medium (500), not Normal (400). The "18pt" variant uses an open lowercase "g" (double-storey in standard Inter) which better matches the official app.

## Packaging

- `Packager.toml` at workspace root configures `cargo-packager` for .app bundle, .dmg, .msi, icons, and signing.
- `tools/packager/` is a thin wrapper crate that calls `cargo_packager::cli::run`.
- Build distributable: `cargo run --bin packager`.
- Window icon set via `window::Settings::icon` using `assets/icon.png` + `image` crate.

## Design Notes

- All colors are theme-swappable. Light is the default theme; dark/light toggle via Help > About Bitwarden.
- Account switcher uses DropDown overlay with `BelowRight` alignment.
- Sidebar uses BWI icons (not Bootstrap Icons) to match the official app.
- SVG logo antialiasing: Iced's resvg rasterizer doesn't match browser-quality. Container constrains visual size while SVG fills available width.
- `styled_card()` has a subtle shadow (black 20% opacity, offset 0,1, blur 2).
- On macOS when `should_use_custom_menu_bar()` is false, `title_bar::view_empty()` renders an empty colored strip (no buttons or drag area).

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

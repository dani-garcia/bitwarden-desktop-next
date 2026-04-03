# bitwarden-desktop-native

Lightweight Rust alternative to the Bitwarden desktop app using Iced 0.14 GUI framework.

## Quick Start

```bash
cargo run                      # Starts on login screen
cargo clippy                   # Lint check (must pass clean)
```

## Project Context

- **UI-only stub** — no business logic, crypto, or API calls. A separate SDK (`PasswordManagerClient`) will handle that later.
- **Multi-user** — state holds `HashMap<UserId, UserSession>`, mirrors the future SDK model.
- **Iced 0.14** — Elm architecture (message-driven updates). Widgets: `button`, `text_input`, `container`, `column`, `row`, `scrollable`, `rule`, `pane_grid`. Custom styling via closure-based `.style()` on each widget. `button::Style` requires a `snap: false` field.
- **Custom theme type** — `AppTheme` implements `iced::theme::Base` + widget `Catalog` traits. Dark/light themes with runtime switching. All `.style()` closures receive `&AppTheme` with direct access to semantic color tokens.

## Project Structure

```
src/
  app.rs                        — Root state (30 fields), message routing, view dispatch
  main.rs                       — Application entry point, window config, font loading
  menu.rs                       — Menu structure (labels, shortcuts, enabled states)
  mock.rs                       — Fake users/vault items (each item has a stable `id` field)
  state.rs                      — Core types: AppState, CipherItem, Screen, etc.

  theme/
    mod.rs                      — AppTheme struct, AppColors struct, Base impl, radius constants
    dark.rs                     — Dark palette (colors from actual Bitwarden app)
    light.rs                    — Light palette (placeholder)
    catalog.rs                  — Widget Catalog trait impls for AppTheme

  components/
    mod.rs                      — separator_h(), separator_v(), styled_card() helpers
    buttons.rs                  — primary(), secondary(), ghost(), ghost_icon(), transparent()
    icons.rs                    — Bootstrap Icons + BWI icons with .render()
    account_switcher.rs         — Shared account dropdown (used by login + vault)
    drop_down.rs                — Local fork of iced_aw DropDown with custom alignments

  views/
    mod.rs
    login/
      mod.rs                    — Lock screen view
    vault/
      mod.rs                    — Main vault view with PaneGrid
      widgets/
        sidebar.rs              — Icon rail + expanded nav panel
        item_list.rs            — Vault item table with action icons
        detail_pane.rs          — Item detail view (readonly fields, cards)
        search_bar.rs           — Search input with icon
    title_bar/
      mod.rs                    — Custom title bar (menu labels + window chrome)
      window_chrome.rs          — Platform chrome icons, resize wrapper
      dropdown.rs               — Menu dropdown panels + submenu rendering
```

## Coding Conventions

### Imports
- Group imports: `use crate::{a, b};` not separate `use crate::a; use crate::b;`
- Nest component imports: `use crate::components::{buttons, icons, account_switcher};`
- Import commonly-used iced types directly: `Background`, `Border`, `Color`, `Shadow`, `Alignment` — avoid fully qualified paths in function bodies.
- Use `Alignment::Center` / `Alignment::End` instead of `iced::alignment::Horizontal::Center`.

### Padding
- Use `[v, h]` shorthand: `.padding([8, 16])` — Iced accepts `[u16; 2]` via `.into()`.
- Use `Padding { top, right, bottom, left }` only for asymmetric cases where top != bottom or left != right.
- When all values are integers, omit `.0`: `[8, 16]` not `[8.0, 16.0]`.

### Theming
- All colors come from `AppTheme.colors` (an `AppColors` struct). Never hardcode color values outside `theme/dark.rs` and `theme/light.rs`.
- Radii (`RADIUS_SM`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_PILL`) are structural constants in `theme/mod.rs`, not theme-dependent.
- In `.style()` closures, access colors via `theme.colors.xxx` (the closure receives `&AppTheme`).
- For `text().color()` and `icon.render()`, pass `&AppColors` through function parameters.

### Button Components
- Use `components::buttons::primary(content)`, `secondary(content)`, `ghost(content, ...)`, `ghost_icon(content)`, `transparent(content)` instead of inline `.style()` closures.
- These take `impl Into<Element>` (like iced's `button()`), return `Button` for chaining `.on_press()`, `.padding()`, `.width()`, etc.

### Dropdowns
- Use `components::drop_down::DropDown` (local fork of iced_aw) for overlay dropdowns.
- Custom alignments: `BelowLeft` (menu bars), `BelowRight` (account switcher), `AboveRight`.
- Always set `.on_dismiss(message)` for click-outside-to-close.
- Cross-message dismissal in app.rs: any `Message::TitleBar` closes account dropdown, any `Message::Login`/`Message::Vault` closes title bar menu.

### UI Helpers
- Use `components::separator_h()` / `components::separator_v()` for all separator lines.
- Use `components::styled_card(content)` for card containers with `BACKGROUND` bg and `RADIUS_LG` corners.

## Reference App

The official Bitwarden app is in `clients/` (git submodule). Key locations:
- Button styles: `clients/libs/components/src/button/button.component.ts` (12px border-radius, Tailwind)
- Combined logo SVG: `clients/libs/assets/src/svg/svgs/password-manager.ts`
- Background illustrations: `clients/libs/assets/src/svg/svgs/background-{left,right}-illustration.ts`
- Lock icon: `clients/libs/assets/src/svg/svgs/lock.icon.ts`
- BWI icon codepoints: `clients/libs/angular/src/scss/bwicons/styles/style.scss`
- BWI font: `clients/libs/angular/src/scss/bwicons/fonts/bwi-font.ttf`
- Menu entries + shortcuts + enabled states: `clients/apps/desktop/src/main/menu/menu.*.ts`
- Desktop layout: `clients/apps/desktop/src/app/layout/desktop-layout.component.html`

## Docs

- `docs/architecture.md` — Structure, state model, SDK integration plan
- `docs/design-reference.md` — Official app findings: button styles, colors, logos, illustrations
- `docs/changelog.md` — Visual changes per UI iteration with screenshots
- `docs/decisions.md` — Framework choice, naming, architecture decisions with rationale

## Design Notes

- All colors are theme-swappable. Dark/light mode toggle via Help > About Bitwarden.
- Account switcher shows server URL (e.g. "bitwarden.com") with a DropDown overlay.
- Match the new Bitwarden component library styles, not the legacy desktop SCSS.
- SCSS color values don't match actual rendered colors — always verify with a color picker.
- Sidebar uses BWI icons (not Bootstrap Icons) to match the official app.
- SVG logo antialiasing: Iced's resvg rasterizer doesn't match browser-quality rendering. Current workaround is to let the container constrain the visual size while the SVG fills available width.

## Pending: State Decentralization (Phase 3)

The remaining major refactoring item. Currently all state (30 fields) lives in `App` and all event handling goes through `App::update()`. The plan:

1. **Extract `LoginView`** — owns `password_input`, `show_password`, `dropdown_open`. Has its own `update()` returning `Vec<LoginAction>` for cross-cutting effects (screen switch, user switch).
2. **Extract `VaultView`** — owns search, filter, selection, sidebar, pane state, item cache. Has its own `update()` returning `Vec<VaultAction>`.
3. **Extract `TitleBarState`** — owns `open_menu`, `open_submenu`. Returns `Vec<TitleBarAction>` (menu actions, window ops).
4. **Slim `App` to ~9 fields** — `state`, `current_theme`, `window_id`, `menu_attached`, `fullscreen`, `maximized`, `login_view`, `vault_view`, `title_bar`.

See `docs/decisions.md` and the plan file for full design details.

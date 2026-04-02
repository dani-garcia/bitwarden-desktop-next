# bitwarden-desktop-native

Lightweight Rust alternative to the Bitwarden desktop app using Iced 0.14 GUI framework.

## Quick Start

```bash
cargo run                      # Starts on login screen
DEV_SCREEN=vault cargo run     # Skips to vault screen (unlocked)
cargo clippy                   # Lint check (must pass clean)
```

## Project Context

- **UI-only stub** — no business logic, crypto, or API calls. A separate SDK (`PasswordManagerClient`) will handle that later.
- **Multi-user** — state holds `HashMap<UserId, UserSession>`, mirrors the future SDK model.
- **Iced 0.14** — Elm architecture (message-driven updates). Widgets: `button`, `text_input`, `container`, `column`, `row`, `scrollable`, `rule`, `pane_grid`. Custom styling via closure-based `.style()` on each widget. `button::Style` requires a `snap: false` field.

## Key Files

- `src/app.rs` — Root state, message routing, view dispatch, PaneGrid state for resizable splits. All state changes go through `update()`.
- `src/menu.rs` — Menu structure (labels, shortcuts, enabled states). Native `muda` on macOS, custom-drawn on Windows/Linux.
- `src/icons.rs` — Bootstrap Icons + Bitwarden Icons (bwi) integration. `Icon`/`BwiIcon` types with `.render()`. Bootstrap constants auto-generated from CSS by `build.rs`; BWI constants defined manually from `clients/libs/angular/src/scss/bwicons/styles/style.scss`.
- `src/theme.rs` — Color constants and border-radius constants. Includes `RADIUS_SM/MD/LG/PILL`. Note: `HEADER_BG == BORDER`, `CARD_BG == SIDEBAR_SELECTED`, `TEXT_MUTED == TABLE_HEADER` (same values, different semantic names).
- `src/views/login.rs` — Lock screen. `src/views/vault.rs` — Main vault view with PaneGrid for item list / detail pane.
- `src/widgets/common.rs` — Shared UI helpers: `separator_h()`, `separator_v()`, `hover_button_style()`, `styled_card()`.
- `src/widgets/` — Reusable components: sidebar, item_list, detail_pane, search_bar, account_switcher, title_bar.
- `src/mock.rs` — Fake users/vault items (each item has a stable `id` field). `src/state.rs` — Core types.

## Coding Conventions

### Imports
- Group imports: `use crate::{a, b};` not separate `use crate::a; use crate::b;`
- Nest widget imports: `use crate::widgets::{sidebar::{self, SidebarMessage}, item_list::ItemListMessage};`
- Import commonly-used iced types directly: `Background`, `Border`, `Color`, `Shadow`, `Alignment` — avoid fully qualified `iced::Background::Color(...)` in function bodies.
- Use `Alignment::Center` / `Alignment::End` instead of `iced::alignment::Horizontal::Center`.

### Padding
- Use `[v, h]` shorthand: `.padding([8, 16])` — Iced accepts `[u16; 2]` via `.into()`.
- Use `Padding { top, right, bottom, left }` only for asymmetric cases where top != bottom or left != right.
- When all values are integers, omit `.0`: `[8, 16]` not `[8.0, 16.0]`.

### UI Patterns
- Use `common::separator_h()` / `common::separator_v()` for all separator lines.
- Use `common::hover_button_style(status, is_active, active_bg, radius)` for transparent-bg buttons with hover.
- Use `common::styled_card(content)` for card containers with `BACKGROUND` bg and `RADIUS_LG` corners.
- Use theme radius constants (`RADIUS_SM`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_PILL`) instead of magic numbers.

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

## Screenshots

`screenshot-all.ps1` captures both screens using iced's built-in `window::screenshot()` API.

```powershell
powershell -ExecutionPolicy Bypass -File screenshot-all.ps1 -OutDir "img/versionN"
```

## Design Notes

- All colors must be theme-swappable (light/dark mode support planned).
- Account switcher should show server URL (e.g. "bitwarden.com") and a dropdown arrow.
- Match the new Bitwarden component library styles, not the legacy desktop SCSS.
- SCSS color values don't match actual rendered colors — always verify with a color picker.
- Sidebar uses BWI icons (not Bootstrap Icons) to match the official app.
- SVG logo antialiasing: Iced's resvg rasterizer doesn't match browser-quality rendering. Current workaround is to let the container constrain the visual size while the SVG fills available width.

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
- **Iced 0.14** — Elm architecture (message-driven updates). Widgets: `button`, `text_input`, `container`, `column`, `row`, `scrollable`, `rule`. Custom styling via closure-based `.style()` on each widget. `button::Style` requires a `snap: false` field.

## Key Files

- `src/app.rs` — Root state, message routing, view dispatch. All state changes go through `update()`.
- `src/menu.rs` — Menu structure (labels, shortcuts, enabled states). Native `muda` on macOS, custom-drawn on Windows/Linux.
- `src/icons.rs` — Bootstrap Icons integration. `Icon` type with `.render()`. Constants auto-generated from CSS by `build.rs`.
- `src/theme.rs` — Color constants. **TODO**: refactor to trait-based theme system for light/dark mode.
- `src/views/login.rs` — Lock screen. `src/views/vault.rs` — Main vault view.
- `src/widgets/` — Reusable components: sidebar, item_list, search_bar, account_switcher, menu_bar.
- `src/mock.rs` — Fake users/vault items. `src/state.rs` — Core types.

## Reference App

The official Bitwarden app is in `clients/` (git submodule). Key locations:
- Button styles: `clients/libs/components/src/button/button.component.ts` (12px border-radius, Tailwind)
- Logo SVG: `clients/apps/web/src/images/logo-white.svg`
- Background illustrations: `clients/libs/assets/src/svg/svgs/background-{left,right}-illustration.ts`
- Lock icon: `clients/libs/assets/src/svg/svgs/lock.icon.ts`
- Menu entries + shortcuts + enabled states: `clients/apps/desktop/src/main/menu/menu.*.ts`
- Desktop SCSS (legacy): `clients/apps/desktop/src/scss/`

## Docs

- `docs/architecture.md` — Structure, state model, SDK integration plan
- `docs/design-reference.md` — Official app findings: button styles, colors, logos, illustrations
- `docs/changelog.md` — Visual changes per UI iteration with screenshots
- `docs/decisions.md` — Framework choice, naming, architecture decisions with rationale

## Screenshots

`screenshot.ps1` captures the app window by title. Images stored in `img/version{N}/`.

```powershell
powershell -ExecutionPolicy Bypass -File screenshot.ps1 -OutputPath "img/versionN/Login.png"
powershell -ExecutionPolicy Bypass -File screenshot-all.ps1 -OutDir "img/versionN"
```

## Design Notes

- All colors must be theme-swappable (light/dark mode support planned).
- Account switcher should show server URL (e.g. "bitwarden.com") and a dropdown arrow.
- Match the new Bitwarden component library styles, not the legacy desktop SCSS.
- SCSS color values don't match actual rendered colors — always verify with a color picker.

# bitwarden-desktop-next

Lightweight Rust alternative to the Bitwarden desktop app using the Iced 0.15 GUI framework (git-pinned — 0.15 is still pre-release on crates.io).

UI-only stub: crypto, API, auth, vault decrypt run through the real `bitwarden-*` SDK crates (git-pinned via `sdk-internal`). Multi-user (one `PasswordManagerClient` per user in `ClientManager`). Custom `AppTheme` with light default, dark/light toggle. Dual menu system (custom on Windows/Linux, native muda on macOS). Cargo workspace; `default-members` skip `tools/*`.

## Quick Start

```bash
cargo run                                     # Loading screen → login
cargo clippy                                  # Lint check (must pass clean)
cargo run --bin packager                      # Package .app/.dmg/.msi
cargo run -p fake-data                        # Regenerate per-user SQLite + mock.json under data/
```

## Dev Modes

```bash
DEV_BOTH_MENUS=1 cargo run                                            # Native + custom menus side-by-side
RUST_LOG=bitwarden_desktop_next=debug,bitwarden_core=debug cargo run  # Full SDK tracing
```

`--gpu` is gated by `cfg!(feature = "gpu")` in `main.rs`; enable wgpu by uncommenting `"wgpu"` in iced features in `crates/desktop/Cargo.toml` and rebuilding.

## Docs (READ BEFORE CHANGES)

- [docs/architecture.md](docs/architecture.md) — project structure, state model, compositional MVU pattern, menu/icon systems, **"How to Add a New View"** recipe.
- [docs/decisions.md](docs/decisions.md) — rationale for framework choice, theme system, overlay approach, MVU, startup lazy-load, packaging, etc. Read first when changing an architectural choice.
- [docs/todo.md](docs/todo.md) — pending work tiers, deferred / upstream items.
- [docs/design-reference.md](docs/design-reference.md) — official-app findings: button styles, palette, fonts, menu structure, screens.
- [docs/skills/](docs/skills/) — periodic review outputs from code-architect / code-explorer / code-reviewer / simplify.

## Coding Conventions

### Imports
- Group imports: `use crate::{a, b};` not separate `use crate::a; use crate::b;`.
- Nest component imports: `use crate::components::{buttons, icons, account_switcher};`.
- Import commonly-used iced types directly: `Background`, `Border`, `Color`, `Shadow`, `Alignment`.

### Padding
- Use `[v, h]` shorthand: `.padding([8, 16])`.
- Use `Padding { top, right, bottom, left }` only for asymmetric cases where neither pair is symmetric.

### Theming
- All colors come from `AppTheme.colors`. Never hardcode outside `theme/dark.rs` and `theme/light.rs`.
- Radii (`RADIUS_SM/MD/LG/PILL`) are structural constants, not theme-dependent.
- In `.style()` closures: `theme.colors.xxx`. For `text().color()` / `icon.render()`: pass `&AppColors`.
- Sidebar-specific tokens: `nav_text` (white in both themes) and `nav_item_hover`.
- Font sizes: 5 values — 12, 14, 16, 18, 28.
- Prefer builder form: `Border::default().rounded(r).color(c).width(w)` and `container::Style::default().background(c).border(b)` over `{ ..Default::default() }` struct literals.

### Button Components
- Use `components::buttons::{primary, secondary, ghost, ghost_icon, transparent}(content)`.
- Takes `impl Into<Element>`, returns `Button` for chaining.
- `ghost(content, is_active, active_bg, hover_bg, radius)` — full knobs for sidebar + lists.
- `ghost_icon(content, hover_bg)` — shorthand for icon-only transparent buttons (RADIUS_SM, no active state).

### Icons
- `icon.render(size, color)` — renders as Element for general use.
- `icon.input_icon(size, side)` — builds `text_input::Icon` for `TextInput::icon()`.
- `icon.char()` is `pub(crate)` — used by `toast` and `window_chrome` where the codepoint needs to compose with other text styling.

### Dropdowns
- Use `components::drop_down::DropDown` with custom alignments: `BelowLeft`, `BelowRight`, `AboveRight`.
- Always set `.on_dismiss(message)` for click-outside-to-close.
- Cross-view dismissal lives at the App router (see [docs/architecture.md](docs/architecture.md) → "Router + Cross-View Dismissal"). Sub-views provide `dismiss_dropdowns()` helpers; don't make sub-views aware of each other.

### Dead Code
- Use `#[expect(dead_code)]` (not `#[allow]`) — warns if suppression becomes unnecessary.
- Exception: `components/icons.rs` uses `#![allow(dead_code)]` at file scope because the generated `bootstrap_icons_generated.rs` includes many unused `Icon` constants.

### Widget IDs
- Widget IDs referenced from more than one file live as `pub const <NAME>: widget::Id = widget::Id::new("...")` in the widget's own module (e.g. `views/vault/widgets/search_bar.rs::SEARCH_ID`).

## Iced Gotchas

- `button::Style` / `rule::Style` require `snap: false` — missing it produces a confusing compile error pointing at the struct literal, not the field.
- `view()` returns `Element<'_, M, Theme>` borrowing from `&self`. Locally computed `Vec`s can't flow into the returned Element — use cached fields on the struct (`ViewCache`, per-user `ItemCache`).
- iced overlays only support ONE level — a `DropDown` inside another `DropDown`'s overlay won't render its own overlay. Submenus must be part of the same overlay content (e.g. `row![main_panel, submenu]`).
- **Nested container backgrounds mask parent border-radius**: iced clips a container's own background fill to its border radius, but a child container's rectangular fill paints right over the rounded edge. Put the rounded fill on the innermost container, or drop inner backgrounds.
- **Stack doesn't cull or clip**: `iced::widget::stack` lays out every child fully at the same bounds. Two heavy subtrees stacked at full window width ≈ 2× layout cost per frame. Prefer exclusive branches in `view()` over layering when only one is shown at a time. For window-level overlays (sheets, modals), compose at the App level so the underlying view can take a cheaper exclusive branch.
- External blocking receivers (muda menu events, tray clicks, named-pipe wake signals) go through a dedicated std thread that forwards into a tokio channel, exposed to iced as `Subscription::run(fn_pointer)`. Avoid `iced::time::every()` as a polling crutch for these — it wakes `App::update` 60×/s even when idle. See `menu::muda_event_stream` / `tray::click_stream` / `instance_lock::wake_stream` for the shape.
- For per-widget animation prefer the self-driving `RedrawRequested` + `shell.request_redraw()` (or `request_redraw_at` when the cadence must be explicit) pattern — it avoids running full `App::update` cycles just to redraw one widget.
- `widget::operation::focus(Id)` sets input focus without a message round-trip; it's a `Task`-returning operation.
- `iced_aw` (0.13) is kept as a dependency with `default-features = false` purely so the source is in the cargo registry cache for reference — we use our own `drop_down.rs` fork at runtime.
- `AppTheme` is cloned every time iced calls `App::theme(window_id)`. Keep the struct cheap: `name` is `&'static str`, `colors` is `Copy`.

## Source Reference Locations

When investigating iced internals, the git-pinned iced checkouts live at:

- iced core: `~/.cargo/git/checkouts/iced-*/<rev>/core/src/`
- iced widgets: `~/.cargo/git/checkouts/iced-*/<rev>/widget/src/`
- iced futures (runtime / backend / executors): `~/.cargo/git/checkouts/iced-*/<rev>/futures/src/`
- iced_aw (registry, old version kept as reference): `~/.cargo/registry/src/*/iced_aw-0.13.1/src/`
- muda: `~/.cargo/registry/src/*/muda-0.18*/src/`

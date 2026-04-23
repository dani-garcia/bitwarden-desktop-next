# Architecture

This is the prescriptive guide for the `bitwarden-desktop-next` crate. It defines the module layout, the dependency direction, and the contract each module type must satisfy. Treat the rules as rigid. Deviations need a short comment explaining why.

## Overview

`bitwarden-desktop-next` is a lightweight Rust alternative to the Electron/Angular Bitwarden desktop app, built on the **Iced 0.15** GUI framework (git-pinned; 0.15 is still pre-release on crates.io). It wraps the real `bitwarden-*` SDK crates for all crypto, auth, and vault work. The UI layer is in Rust; there is no JavaScript.

### Tech stack

- **Language**: Rust 2024
- **GUI**: Iced 0.15 (git) with `tiny-skia` default backend; `wgpu` opt-in behind the `gpu` Cargo feature.
- **SDK**: `bitwarden-core` / `bitwarden-crypto` / `bitwarden-pm` / `bitwarden-state` / `bitwarden-vault` git-pinned.
- **Async**: Iced's `tokio` multi-thread runtime.
- **Menus**: `muda` for native macOS, custom-drawn title bar on Windows/Linux.
- **Theme**: Custom `AppTheme` with dark/light palettes, runtime switching, light default.
- **Packaging**: `cargo-packager` via `Packager.toml`.

## Layers

```
┌──────────────────────────────────────────────────┐
│ app/              router, top-level Message      │
├──────────────────────────────────────────────────┤
│ views/<name>/     per-screen state + update + UI │
├──────────────────────────────────────────────────┤
│ components/       shared UI atoms                │
├──────────────────────────────────────────────────┤
│ services/<name>/  platform + stateful services   │
├──────────────────────────────────────────────────┤
│ domain.rs + theme/  types + visual contract      │
└──────────────────────────────────────────────────┘
```

**Invariant:** a module may import only from layers strictly below its own. Within a layer, no sibling-to-sibling imports — sibling views communicate only through `App`'s router; sibling services communicate only through well-defined APIs.

**Enforcement:** Rust visibility is the primary mechanism. Within a layer, internal types and helpers are **never** left as bare `pub` — that accidentally exports them crate-wide and breaks layer isolation. Use the narrowest level that works:

- `pub(in crate::views::<view>)` — visible only inside one view's subtree (including its `widgets/`). Use for view-local domain enums and state structs.
- `pub(super)` — visible to the parent module. Use for sub-module internals.
- `pub(crate)` — visible across the crate. Use only for the archetype's documented public API (e.g., `VaultView`, `VaultMessage`, `VaultEvent`, `view()`, `update()`, `new()`, named task factories, and the event-handler carve-out).
- private (no `pub`) — default for struct fields and internal helpers.

The compiler refuses cross-layer imports at the `use` line once visibility is set correctly. A grep-based CI check is available as a backstop. When a `pub` enum variant must wrap a narrower type (e.g., `VaultMessage::Sidebar(SidebarMessage)` carrying view-local payloads), silence the resulting `private_interfaces` lint with a scoped `#[allow]` and a one-line comment pointing to the reason.

**One carve-out:** each view's `handler.rs` is the single view-layer file allowed to import `crate::app::App`. Its job is to translate the view's events into app-level effects (screen flips, user switches, toast pushes); locking it to the view that owns the events keeps related code together.

## Directory Layout

```
crates/desktop/src/
├── main.rs                         # entry: font load, backend select, iced::daemon
├── domain.rs                       # truly cross-cutting types: UserId, Screen, UnlockMethod(s)
├── paths.rs                        # data_dir() — static helpers
├── assets.rs                       # include_bytes! blobs
│
├── app/                            # router layer
│   ├── mod.rs                      # App struct, new/update/view/subscription
│   ├── message.rs                  # top-level Message + WindowMessage + SystemMessage
│   ├── ctx.rs                      # UpdateCtx, RenderCtx bundle definitions
│   ├── helpers.rs                  # refresh_cache, menu_state, platform bits
│   └── handlers/platform.rs        # window lifecycle, system events, menu/tray dispatch
│
├── services/                       # platform + stateful cross-view services
│   ├── sdk/                        # ClientManager (bitwarden SDK facade)
│   ├── clipboard/                  # ClipboardManager + sensitivity
│   ├── favicon/                    # FaviconService + fetch stream
│   ├── menu/                       # muda binding + MENUS + MenuAction
│   ├── tray/                       # TrayHandle + click stream
│   ├── i18n/                       # fluent loader + fl! macro target
│   ├── instance_lock/              # single-instance guard + wake stream
│   ├── preferences/                # per-user UserPreferences
│   └── settings/                   # Settings load/save
│
├── components/                     # shared UI atoms
│   ├── buttons.rs                  # primary/secondary/ghost/ghost_icon/icon_button
│   ├── icons.rs / inputs.rs / drop_down.rs / spinner.rs / virtual_list.rs
│   ├── bottom_sheet.rs / modal.rs / totp.rs
│   ├── toast/                      # folder: multi-file component
│   └── account_switcher.rs
│
├── views/                          # one folder per screen (team boundary)
│   ├── login/
│   │   ├── mod.rs
│   │   ├── handler.rs              # handle_login_event(&mut App, ev) — layer carve-out
│   │   └── (view-private subviews) # unlock.rs, login_email.rs, login_password.rs, …
│   ├── vault/
│   │   ├── mod.rs
│   │   ├── handler.rs
│   │   └── widgets/                # view-private custom widgets
│   ├── settings/, title_bar/       # same shape
│   └── about/                      # stateless exception (single mod.rs + handler.rs)
│
└── theme/                          # visual contract (palette + iced catalog)
```

## Module Archetypes

### View (`views/<name>/`)

Always a folder. Expected file set:

| File | Contents | Soft cap |
|---|---|---|
| `mod.rs` | `pub use` re-exports; public entry (`new`, `update`, `view`) | 150 |
| `state.rs` | `{View}View` struct + view-local domain types | 300 |
| `message.rs` | `{View}Message`, `{View}Event`, helper enums | 200 |
| `update.rs` | `impl {View}View::update` (takes `&UpdateCtx`) + helpers | 500 |
| `view.rs` | `impl {View}View::view` (takes `&RenderCtx`) + subview builders | 500 |
| `handler.rs` | `impl App { pub(crate) fn handle_{view}_event(...) }` | 200 |
| `widgets/` | view-private custom widgets | — |

The state/message/update/view split applies once the view's `mod.rs` approaches the soft cap — smaller views can keep everything in `mod.rs` plus a `handler.rs`. Trigger rule: when `mod.rs` crosses ~400 lines **and** holds two or more of `{state fields and methods, message/event enums, update match, view builders}`, extract in this order: `state.rs` first (data + struct), then `message.rs` (enums), then `update.rs` and `view.rs` last (since they're the largest). The goal is every file under ~500 lines.

**Public surface of a view** (the full crate-visible API, callable from `App`):
- `{View}View` struct (fields stay private; construction via `new()`).
- `{View}Message`, `{View}Event` enums.
- `{View}View::new()`, `update()`, `view()`, `dismiss_dropdowns()`.
- Named task factories (e.g. `load_list_task`). Convention: top-level `pub(crate) fn {action}_task(...)` in `update.rs` (or `mod.rs` if the view hasn't been split). Called from `App` helpers when a task needs to be kicked off outside the view's own `update()` (e.g., post-unlock vault reload). Factory functions never take `&mut self` — they take owned + borrowed inputs and return `Task<{View}Message>`.
- `handle_{view}_event` (defined inside `handler.rs` as `impl App`, marked `pub(crate)`).

Everything else stays `pub(super)` or private. Struct fields default to private.

### Stateless view exception (`about`)

A view with no runtime state skips the `{View}View` struct and the `update/new` methods. Its mod.rs exports just:

- `pub(crate) enum {View}Message` — the outbound actions (e.g. `AboutMessage::CopyInfo`, `AboutMessage::Close`).
- `pub(crate) fn view(ctx: &RenderCtx) -> Element<{View}Message, AppTheme>`.
- Any pure helpers the message handler needs (e.g. `info_string()`).

The `handler.rs` still exists and implements `impl App { fn handle_{view}_message(...) }`. Use this pattern only when the view is strictly non-interactive beyond dispatching messages and has no input state, hover state, or async work — for example, the About window. If state is ever added, convert to the full archetype.

### Component (`components/<name>.rs` or `components/<name>/`)

Single file by default; folder only when the component needs multiple files (toast).

- Public API: `(data, callbacks, &RenderCtx_or_&AppColors) -> Element`.
- Internal state (reveal toggle, hover, countdown, fade, progress) lives **inside** the component via iced's `Component` trait, widget-tree state (`Tree` param), or `Rc<Cell<_>>` for values that style closures read during draw.
- **Forbidden:** requiring the parent to pass `is_revealed: bool` or `progress: f32` or any transient UI state. The parent provides data; the component renders.
- Components may not import from `views/` or `app/`. They may receive `&RenderCtx` but never `&UpdateCtx`.

Reference implementations:
- `components/toast/` — self-driving animations via `RedrawRequested` + `Rc<Cell<ToastVisuals>>`.
- `components/spinner.rs` — self-animating ring via `shell.request_redraw()`.
- `components/totp.rs` — countdown widget with internal timer.

### Service (`services/<name>/`)

Always a folder:

| File | Contents |
|---|---|
| `mod.rs` | Public API: struct + `pub(crate)` methods |
| `<impl>.rs` | Private implementation (workers, caches, IO) |

- Constructed once in `App::new`.
- Injected to views via `UpdateCtx` (for update paths) or `RenderCtx` (for render paths where the service is visible — e.g. `FaviconService`).
- May not import from `views/`, `components/`, or `app/`. May import from `domain`, `theme`, and other services (acyclic).

### Domain (`domain.rs`)

- Cross-cutting types only: `UserId`, `Screen`, `UnlockMethod`, `UnlockMethods`.
- View-local domain lives in the owning view's `state.rs` or `mod.rs` (e.g. `SidebarFilter`, `NavSection`, `SidebarMode` in `views/vault/mod.rs`; the login view's local types in `views/login/`).
- No logic beyond `impl` on the types (`Display`, `From`, parse).

## Context Bundles

Signatures with 5+ args use a bundle. Two bundles are defined in `app/ctx.rs`:

```rust
// Update-time: services + session. Used only by view update() paths.
pub struct UpdateCtx<'a> {
    pub client_manager: &'a Arc<ClientManager>,
    pub active_user: Option<&'a UserId>,
}

// Render-time: everything needed to render. Allowed in components.
pub struct RenderCtx<'a> {
    pub colors: &'a AppColors,
    pub favicon: &'a FaviconService,
    pub show_favicons: bool,
    pub window_width: f32,
    pub active_user: Option<&'a UserId>,
    pub active_email: Option<&'a str>,
    pub accounts: &'a [AccountEntry],
}
```

Rules:
- Every view's `update()` takes `&UpdateCtx` and returns `Outcome<{View}Message, {View}Event>` (see next section).
- Every view's `view()` and its internal render helpers take `&RenderCtx`.
- Components take `&RenderCtx` (or a narrower arg like `&AppColors` if they only need colors).
- Components **never** take `UpdateCtx` — the type isn't reachable from `components/`, so the compiler prevents it.
- Any new service added to `UpdateCtx` or `RenderCtx` becomes available to all views with no signature churn.

## Update Outcome

Every view implements a one-line [`ViewTypes`](crates/desktop/src/app/ctx.rs) trait that ties it to its own `Message` + `Event`:

```rust
impl ViewTypes for VaultView {
    type Message = VaultMessage;
    type Event = VaultEvent;
}
```

That lets every view's `update()` return `Outcome<Self>` instead of repeating `Outcome<FooMessage, FooEvent>` at every handler signature:

```rust
pub enum Outcome<V: ViewTypes> {
    None,
    Task(Task<V::Message>),
    Event(V::Event),
}
```

Three mutually-exclusive cases. Call sites use named constructors:

```rust
Outcome::None                                           // fallthrough / no-op
return Outcome::event(LoginEvent::Unlocked { uid });    // bubble a fact
return Outcome::spawn(async move { ... }, |r| msg);     // spawn an SDK task
return Outcome::from_option(maybe_event);               // Some/None → Event/None
```

No `From<Event>` blanket impl — Rust's coherence rules conflict with the reflexive `From<T> for T` from stdlib. The named `Outcome::event(e)` constructor is the accepted equivalent.

App-side routing uses the `dispatch` method:

```rust
Message::Vault(m) => self
    .vault_view
    .update(m, &uctx)
    .dispatch(Message::Vault, |e| self.handle_vault_event(e)),
```

**The `Both(Task, Event)` case is deliberately excluded.** When a view wants both to spawn a task *and* bubble an event (typically "SDK op succeeded → reload list + show success toast"), it emits a richer event (e.g. `VaultEvent::ItemSaved { uid }`) and App's handler fires both effects. The view reports the fact; App decides the follow-up.

## Messages + Events

### Suffix discipline

- `*Message` — inbound (widget events + async completions). The `update(msg)` argument. Imperative widget events (`LoginMessage::Unlock`) and past-tense async completions (`VaultMessage::ListLoaded`) both belong here.
- `*Event` — outbound fact a sub-view bubbles up. Describes what *happened*, not what to do next. Examples: `LoginEvent::Unlocked`, `VaultEvent::ToastRequested`, `SettingsEvent::Applied`.
- `*Action` — discriminant for an abstract operation: `MenuAction::Quit`, `WindowAction::Minimize`. Use as a payload on an `Event` when the view is requesting an app-level effect.

Private widget-internal enums under `components/` don't participate in the router; they aren't governed by these conventions.

### Compositional MVU

Each view implements:

```rust
pub fn update(
    &mut self,
    msg: {View}Message,
    ctx: &UpdateCtx,
) -> (Task<{View}Message>, Option<{View}Event>);
```

- **`Task<{View}Message>`** carries any async work the view kicked off; the App router lifts it via `.map(Message::{View})`.
- **`Option<{View}Event>`** carries a declarative fact for App to route; `None` means the view handled everything locally.

App's `handle_{view}_event` methods (defined in the view's `handler.rs`) translate each event into concrete side effects: screen switches, user swaps, toast pushes, menu actions, follow-on tasks.

Async callbacks live inside sub-messages, not at the top level. `LoginMessage::UnlockCompleted(uid, Result)` is handled by `LoginView::update` (stale-check, clear `unlock_in_progress`, emit `LoginEvent::Unlocked` or a sanitized error toast).

### Top-level Message

```rust
pub enum Message {
    Login(LoginMessage),
    Vault(VaultMessage),
    TitleBar(TitleBarMessage),
    About(AboutMessage),
    Settings(SettingsMessage),
    Window(WindowMessage),      // per-window OS events
    System(SystemMessage),      // MudaEvent, TrayClick, ThemeChanged, etc.
    Favicon(FaviconMessage),    // fetch completions
}
```

### Router + cross-view dismissal

`App::update` is a pre-match dismissal block followed by the per-variant router:

```rust
Message::Vault(m) => {
    let (task, ev) = self.vault_view.update(m, &uctx);
    let task = task.map(Message::Vault);
    let ev_task = ev
        .map(|e| self.handle_vault_event(e))
        .unwrap_or_else(Task::none);
    Task::batch([task, ev_task])
}
```

The pre-match block is **load-bearing**: any login/vault message dismisses the title-bar menu, and any title-bar message dismisses the login/vault dropdowns. Without it, an accidental menu click from the vault would leave the account-switcher dropdown open.

## Cross-Cutting Rules

1. **Dependency direction** — enforced via visibility (`pub(super)` / `pub(crate)`). A view's internals are `pub(super)`; a component's helpers are `pub(super)`. Only documented archetype APIs are `pub(crate)`.

2. **Size budget** — soft cap ~500 lines per file (cognitive load + git-diff readability + parallel work without conflicts). Trigger the archetype split when a file crosses ~400 lines and contains two or more separable concerns. Split in the order described in the View archetype above (state → message → update/view). Pure widget files (under `widgets/`) may exceed the cap temporarily; prefer extracting sub-sections (e.g., `cipher_form/sections/`) over folding growth back into `mod.rs`.

3. **No mandatory test policy** — the app is a POC. Tests are opt-in when a piece of logic is gnarly enough to warrant one. Revisit once the codebase stabilizes.

4. **Crate root** — only `main.rs`, `domain.rs`, `paths.rs`, `assets.rs`. Everything else has a categorized home.

5. **Style / theming / i18n**
   - Colors come from `AppTheme.colors` via `RenderCtx` or `&AppColors`. Never hardcode outside `theme/dark.rs` and `theme/light.rs`.
   - Radii (`RADIUS_SM/MD/LG/PILL`) are structural constants, not theme-dependent.
   - Font sizes: 5 values — 12, 14, 16, 18, 28.
   - Localized strings go through the `fl!` macro (compile-time checked against `assets/i18n/en/*.ftl`).

6. **`update()` arm style** — when a match has many state-mutation arms and a few emitter arms, use tail-fallthrough: each state arm is a plain statement (no tuple constructor), emitter arms use `return`, and the function ends with a single `(Task::none(), None)` after the match. This keeps the common case terse and makes `return` a visual marker for "this arm bubbles out". Reserve the expression-match style (explicit tuple in every arm) for cases where almost every arm emits, or where the match is a dispatch delegating to handlers that already return tuples.

   ```rust
   // Tail-fallthrough — most arms mutate, a few emit:
   match msg {
       Message::Toggle => self.open = !self.open,
       Message::Reset => self.reset(),
       Message::Save => return (save_task(), None),
       Message::Cancel => return (Task::none(), Some(Event::Cancelled)),
   }
   (Task::none(), None)
   ```

## How to Add a New View

Follow this recipe. End-to-end you'll touch your own view directory plus four mechanical adds in `app/mod.rs` and one in each matching list.

### 1. Create the view folder

```
crates/desktop/src/views/generator/
├── mod.rs              # GeneratorView struct + GeneratorMessage + GeneratorEvent + new/update/view
├── handler.rs          # impl App { fn handle_generator_event(...) -> Task<Message> }
└── widgets/            # optional: custom widgets specific to this view
```

Minimal `mod.rs` skeleton:

```rust
mod handler;

use iced::{Element, Task};

use crate::{
    app::{RenderCtx, UpdateCtx},
    theme::AppTheme,
};

#[derive(Debug, Clone)]
pub enum GeneratorMessage { /* widget events + async completions */ }

#[derive(Debug, Clone)]
pub enum GeneratorEvent { /* outbound facts for App */ }

pub struct GeneratorView { /* local state */ }

impl GeneratorView {
    pub fn new() -> Self { Self { /* ... */ } }

    pub fn update(
        &mut self,
        msg: GeneratorMessage,
        _ctx: &UpdateCtx,
    ) -> (Task<GeneratorMessage>, Option<GeneratorEvent>) {
        // match msg { ... }
        (Task::none(), None)
    }

    /// Close any local overlays when a message destined for another view arrives.
    pub fn dismiss_dropdowns(&mut self) {}

    pub fn view<'a>(
        &'a self,
        _ctx: &RenderCtx<'a>,
    ) -> Element<'a, GeneratorMessage, AppTheme> {
        iced::widget::text("Generator").into()
    }
}
```

Register in `crates/desktop/src/views/mod.rs`:

```rust
pub(crate) mod generator;
```

### 2. Add the Message variant in `app/message.rs`

```rust
pub enum Message {
    // existing variants...
    Generator(GeneratorMessage),
}
```

### 3. Add the view struct field on `App` in `app/mod.rs`

```rust
pub struct App {
    // existing fields...
    pub(super) generator_view: generator::GeneratorView,
}
```

Initialize in `App::new()`: `generator_view: generator::GeneratorView::new(),`.

### 4. Add the router arm in `App::update`

```rust
Message::Generator(m) => {
    let (task, ev) = self.generator_view.update(m, &uctx);
    let task = task.map(Message::Generator);
    let ev_task = ev
        .map(|e| self.handle_generator_event(e))
        .unwrap_or_else(Task::none);
    Task::batch([task, ev_task])
}
```

Add `Message::Generator(_)` to the cross-view dismissal pre-match block at the top of `App::update`, so an accidental click into the generator doesn't leave other views' overlays open. It looks like this:

```rust
match &message {
    Message::Login(_) | Message::Vault(_) | Message::Generator(_) => {
        self.title_bar.dismiss_menu();
    }
    Message::TitleBar(_) => {
        self.login_view.dismiss_dropdowns();
        self.vault_view.dismiss_dropdowns();
        self.generator_view.dismiss_dropdowns(); // if the view has overlays
    }
    Message::Settings(_) => self.dismiss_all_overlays(),
    Message::About(_) | Message::Window(_) | Message::System(_) | Message::Favicon(_) => {}
}
```

Only add a `dismiss_dropdowns()` hook for the new view if it owns overlays (dropdowns, popovers). Stateless views and simple forms don't need one.

### 5. Implement the event handler in `views/generator/handler.rs`

```rust
use iced::Task;

use crate::{
    app::{App, Message},
    views::generator::GeneratorEvent,
};

impl App {
    pub(crate) fn handle_generator_event(&mut self, event: GeneratorEvent) -> Task<Message> {
        match event {
            // translate facts into app-level effects
        }
    }
}
```

### 6. Wire a screen switch (if the view owns a screen)

Add `Screen::Generator` in `domain.rs`, branch on it in `App::view_main`, and flip to it from the triggering action.

If the view is a panel/modal within an existing screen, skip this step.

### What this recipe costs in `app.rs`

Exactly four mechanical adds plus one `impl App { fn handle_generator_event }` in the view's own folder:

1. One `Message::Generator(GeneratorMessage)` variant.
2. One `generator_view: GeneratorView` field on `App`.
3. One router arm (calls `.update()` + `handle_generator_event`).
4. One cross-view dismissal entry.

Plus (if the view owns a screen) one `Screen::Generator` variant and one `view_main` branch.

## Menu System

Single `MENUS` static drives both custom and native menus:

- **Custom title bar** (Windows/Linux): renders labels, dropdowns, shortcuts, submenus via `views/title_bar/`.
- **Native muda** (macOS, or `DEV_BOTH_MENUS=1`): `Shortcut::to_accelerator()` adds shortcuts; `NativeMenuHandle` bridges events through `services::menu::muda_event_stream()`. A push callback registered once at startup via `muda::MenuEvent::set_event_handler` fans events out through a `tokio::sync::broadcast::Sender`; each subscription run calls `.subscribe()` for a fresh receiver.
- `MenuState { is_locked, has_accounts, has_lockable_accounts }` — three bools driving `EnabledWhen::Always / Unlocked / HasAccounts / HasLockable`.

## Overlays

Two kinds, both via iced's native overlay system (window-level rendering, not nested in the page layout):

- **Dropdowns** — `components::drop_down::DropDown` (local fork of iced_aw). Custom alignments: `BelowLeft`, `BelowRight`, `AboveRight`. Used for the account switcher and title-bar menus.
- **Toasts** — `components::toast::Manager` wraps the app content and overlays a vertical stack of toasts in the lower-right. Animation state (fade, progress, hover-pause) is driven from the overlay's `update()` via `window::Event::RedrawRequested` ticks — no separate `Subscription`.

Iced overlays support only ONE level — a `DropDown` inside another `DropDown`'s overlay won't render its own overlay. Submenus must be part of the same overlay content (e.g. `row![main_panel, submenu]`).

## Self-Animating Widgets

Pattern: intercept `Event::Window(window::Event::RedrawRequested(now))` in `Widget::update` and request the next frame via `shell`. The widget drives its own redraws without an app-level subscription or dummy message.

Two variants:
- `shell.request_redraw()` — redraw on the display's next frame. Simplest. Used by the spinner.
- `shell.request_redraw_at(now + delta)` — redraw at an explicit instant, capping the rate regardless of display Hz. Used by the toast overlay and the TOTP countdown.

## Iced Gotchas

- `button::Style` / `rule::Style` require `snap: false` — missing it produces a confusing compile error pointing at the struct literal.
- `view()` returns `Element<'_, M, Theme>` borrowing from `&self`. Locally-computed `Vec`s can't flow into the returned Element — use cached fields (`ViewCache`, per-user `ItemCache`).
- **Nested container backgrounds mask parent border-radius**: put the rounded fill on the innermost container, or drop inner backgrounds.
- **Stack doesn't cull or clip**: `iced::widget::stack` lays out every child fully at the same bounds. Prefer exclusive branches in `view()` over layering when only one is shown at a time.
- External event sources are pushed into `tokio::sync::broadcast` channels at startup and consumed via `Subscription::run(fn_pointer)`. Avoid `iced::time::every()` for external sources.
- `AppTheme` is cloned every time iced calls `App::theme(window_id)`. Keep it cheap: `name` is `&'static str`, `colors` is `Copy`.

## Iced Source Reference

Git-pinned checkouts for investigation:

- iced core: `~/.cargo/git/checkouts/iced-*/<rev>/core/src/`
- iced widgets: `~/.cargo/git/checkouts/iced-*/<rev>/widget/src/`
- iced futures: `~/.cargo/git/checkouts/iced-*/<rev>/futures/src/`
- iced_aw (registry, kept as reference): `~/.cargo/registry/src/*/iced_aw-0.13.1/src/`
- muda: `~/.cargo/registry/src/*/muda-0.18*/src/`

## Key Dependencies

- `iced` (git, 0.15) with `tokio`, `tiny-skia`, `advanced`, `svg`, `image`, `crisp`, `hinting`, `web-colors`, `x11`, `wayland` features
- `muda = "0.18"` — native OS menus
- `iced_aw` (0.13, default-features = false) — kept as local source reference for our `DropDown` fork
- `system-theme = "0.3"` — OS dark/light detection
- `bitwarden-*` — SDK (git-pinned via `[workspace.dependencies]`)
- `tracing` / `tracing-subscriber` — structured logging driven by `RUST_LOG`
- `cargo-packager` — used by `tools/packager`

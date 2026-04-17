# Decisions

## Framework: Iced 0.15

**Decision**: Use Iced as the GUI framework, tracking its pre-release 0.15 via git pin.

**Alternatives considered**: Dioxus (webview-based desktop mode, custom renderer still maturing), Slint (GPL/commercial license concern).

**Rationale**: Pure Rust, no webview, Elm architecture ensures repaints only on state change, ~25k GitHub stars, actively maintained, cross-platform. 0.15 brings a cleaner window/daemon API and builder methods on `Border` / `container::Style` that remove much of the `..Default::default()` boilerplate.

## Renderer: tiny-skia by default

**Decision**: Default to tiny-skia (CPU renderer). The wgpu GPU backend is available behind a feature flag.

**Why**: wgpu init takes ~500 ms and adds ~9 MB to the binary. A form-based UI does not need it.

**How to enable wgpu**: see [architecture.md](./architecture.md) → "wgpu opt-in".

## Crate Name: bitwarden-desktop-next

**Decision**: Named `bitwarden-desktop-next` to avoid conflict with existing "bitwarden-lite" project.

## Workspace Structure

**Decision**: Organize as a Cargo workspace with the desktop app at `crates/desktop/`, plus `bitwarden_license/*` and `tools/*` as additional members. `default-members = ["crates/*", "bitwarden_license/*"]`.

**Rationale**: Mirrors the pattern used in other Bitwarden Rust projects. Separates the main application from licensed code and developer tooling. `default-members` excludes `tools/*` so `cargo run` / `cargo build` target only the app and licensed code, not the developer tooling; `tools/*` are built explicitly via `cargo run -p fake-data` / `cargo run --bin packager`.

## UI Only (No Business Logic)

**Decision**: This app is stub UI only. All crypto, API, authentication, and vault operations will come from a separate SDK (`PasswordManagerClient`).

**Rationale**: Separation of concerns. The SDK handles security-sensitive operations; this project handles presentation.

## Multi-User State Model

**Decision**: App state uses per-user keyed data (active_user + items keyed by UserId + per-user `PasswordManagerClient` in `ClientManager`).

**Rationale**: Makes future SDK integration a clean swap. Multiple accounts can be available simultaneously with one active at a time.

## Startup: Lazy Data Load + Spinner

**Decision**: `App::new` returns fast with `client_manager: Arc::new(ClientManager::empty())` and `screen: Screen::Loading`. The 24 MB JSON parse runs on tokio's blocking pool via `spawn_blocking`; when it resolves, `SystemMessage::ClientManagerLoaded(Arc<ClientManager>)` swaps the Arc and transitions to `Screen::Login`.

**Previous approach**: `ClientManager::load()` ran synchronously in `App::new`, blocking the window's first frame for hundreds of milliseconds (up to ~1-2 s on the loadtest account).

**Why changed**: Iced renders the first frame only after `App::new` returns. Blocking inside it means a blank screen while the vault JSON parses. Deferring to a background task with a spinner gives immediate window + spinner feedback.

**Implementation**:

- `ClientManager::empty()` returns a zero-user manager. Existing accessors (`user_ids`, `is_unlocked`, `has_users`, `email`, etc.) all behave correctly on the empty `HashMap`, so no `Option<Arc<ClientManager>>` plumbing is needed across call sites.
- `Task::perform` wraps `tokio::task::spawn_blocking(ClientManager::load)` so the JSON parse is scheduled on tokio's dedicated blocking thread pool, not on an async worker. Iced's `tokio` feature already initializes a multi-threaded runtime with `rt-multi-thread` + `time`, so our crate only needs the `rt` feature of tokio for `spawn_blocking`.
- `SystemMessage::ClientManagerLoaded` arrives, handler swaps the Arc, computes `active_user` from `user_ids().next()`, calls `login_view.show_unlock_for(active_user, &client_manager)`, sets `screen = Screen::Login`.

**Spinner component** (`components/spinner.rs`): self-animating 8-dot ring widget. Intercepts `Event::Window(window::Event::RedrawRequested(now))` in `Widget::update` and calls `shell.request_redraw_at(now + 16ms)` to schedule the next frame. No app-level subscription, no dummy animation message. Same pattern as the toast overlay.

**Per-user lazy cipher parsing (landed via SQLite)**: ciphers now live in per-user `<user_id>.sqlite` files under `data/` and are read through the SDK state registry only when `list_ciphers` / `full_cipher` runs. The original "parse all ciphers at startup" bottleneck is gone.

## Unlock in-flight indicator

**Decision**: During the unlock task, the primary button shows a spinner (same padded height as the text), the password/PIN input is read-only, and the alternate-method + Log out buttons stay visible but inert (no `on_press`). `LoginView.unlock_in_progress: bool` drives the state.

**Rationale**: Nothing disappears between "Unlock clicked" and "unlock completed" — the buttons stay in place but are clearly unavailable. The spinner sits inside the primary button so there's no layout shift. Re-entry guards inside `LoginMessage::Unlock` prevent Enter-spam from queuing multiple unlocks.

**Flag lifecycle**: set true in `LoginMessage::Unlock` just before `Task::perform`; cleared in `UnlockCompleted` (both Ok and Err), `SwitchUnlockMethod`, `reset_to_email_entry`, `show_unlock_for` (covers account switches while a task is in flight).

## Compositional MVU

**Decision**: Each view owns its local state and async work, and returns `(Task<SubMessage>, Option<SubEvent>)` from its `update()` method. App is a thin router. See [architecture.md](./architecture.md) → "Compositional MVU Pattern" for the implementation shape, message flow, and "How to Add a New View" recipe.

**Previous approach**: Sub-views returned `Vec<Action>` with imperative requests; App owned every `Task::perform` call; async completions were orphan top-level `Message` variants.

**Why changed**: top-level `Message` had grown to 13 variants; `App::update()` was 220 lines mixing routing, async dispatch, toast pushes, theme sync, and window chrome. Every new feature touched shared files. `app.rs` became the single-team bottleneck. After the refactor: `Message` shrank to 6, adding a new screen costs four mechanical lines in `app.rs`.

**Prior art**: Halloy — [investigation/halloy/src/buffer.rs:247](../investigation/halloy/src/buffer.rs); cosmic-settings — [investigation/cosmic-settings/cosmic-settings/src/pages/mod.rs:113](../investigation/cosmic-settings/cosmic-settings/src/pages/mod.rs); iced-guide — [investigation/iced-guide/src/app_structure/composition.md](../investigation/iced-guide/src/app_structure/composition.md).

**Kill-switch criteria**: revisit this pattern if (a) sub-views need to nest multiple levels deep — the `Task::map` chain gets awkward past two levels, or (b) we need a middleware router (e.g. undo/redo).

## Colors: Picker Over SCSS

**Decision**: Use color-picked values from the actual running Bitwarden app, not SCSS/CSS variable values.

**Rationale**: The SCSS values in `variables.scss` and Tailwind CSS vars don't match what the app actually renders. For example, `backgroundColor` is `#1f242e` in SCSS but the actual window background is `#070b18`. The SCSS has three background levels and it's unclear which applies where. Always verify with a color picker (Win+Shift+C with PowerToys).

## Menus: Native on macOS, Custom-Drawn Elsewhere

**Decision**: Use `muda` for native menus on macOS only. Draw the menu bar inside iced on Windows and Linux.

**Previous approach**: Used `muda::init_for_hwnd()` on Windows for native Win32 menus.

**Why changed**: Native Win32 menus create a 1px transparent gap between the menu bar and the iced client area (DWM compositor border). This is a known muda issue (#123, fixed for Tauri's webview but not for raw wgpu/tiny-skia rendering). Drawing the menu ourselves eliminates the gap entirely.

**macOS**: Keeps native menus via `init_for_nsapp()` because macOS users expect the system menu bar at the top of the screen.

**Linux**: Custom-drawn (same as Windows) to avoid GTK dependencies.

## Icons: Bootstrap Icons (Self-Bundled)

**Decision**: Bundle Bootstrap Icons TTF font directly and auto-generate Rust constants via `build.rs`.

**Alternatives considered**: `iced_fonts` crate (adds a dependency for something trivial), `iced_aw` icons (claimed Icon enum but didn't have one), Font Awesome, Nerd Fonts, individual SVGs per icon.

**Rationale**: Bootstrap Icons is what the official Bitwarden app uses (`bwi-*` prefixed classes). Doing it ourselves means zero extra dependencies and full control over the version.

**WOFF→TTF**: The Bootstrap Icons release only ships WOFF/WOFF2, but iced's `fontdb` only supports TTF/OTF. We used the `wuff` crate (pure Rust WOFF decoder) once to convert, saved the result in `assets/`, then removed `wuff` from dependencies.

## Font: Inter 18pt Static

**Decision**: Bundle static Inter 18pt font files (`Inter_18pt-Medium.ttf` + `Inter_18pt-Bold.ttf`), default weight Medium (500). Family name: `"Inter 18pt"`. Constants `APP_FONT` / `APP_FONT_BOLD` in `main.rs`.

**Rationale**: The "18pt" optical size variant has an open single-storey "g" matching the Bitwarden app better than variable Inter. Medium (500) matches the app's visual density. Static files avoid the optical-size axis complexity.

## Font Sizes: Consolidated to 5

**Decision**: Use exactly 5 font sizes: 12, 14, 16, 18, 28.

**Rationale**: Fewer sizes create a more consistent visual hierarchy. 12 captions, 14 body/buttons, 16 emphasis, 18 section headers, 28 page titles.

## Button Style: Pill-Shaped (RADIUS_PILL = 20)

**Decision**: Use 20px border-radius (`RADIUS_PILL` constant) for buttons.

**Rationale**: Visual inspection of the actual rendered app shows more rounded buttons than the Tailwind `tw-rounded-xl` (12px) suggests.

## Theme: Custom `AppTheme`

**Decision**: Custom `AppTheme` struct implementing `iced::theme::Base` + widget `Catalog` traits. Dark/light palettes in separate files.

**Status**: Complete. Light theme is the default. `ThemePreference::Light` hardcoded at startup (no settings persistence layer yet). Light palette colors picked from the 2025 Figma mockup. `AppTheme.name: &'static str` (not `String`) so clones are pointer copies, since iced calls `App::theme()` frequently.

**Rationale**: iced's `application()` / `daemon()` is generic over Theme. Custom theme gives `&AppTheme` in all `.style()` closures with direct access to 25 semantic color tokens. No thread_local or registry needed. All colors (including `scrollbar_thumb` used by the default `scrollable` catalog impl) live in `AppColors`.

## Overlays: DropDown Widget (replaces `stack!` pattern)

**Decision**: Use a local fork of iced_aw's `DropDown` widget (`components/drop_down.rs`) with custom alignment variants (`BelowLeft`, `BelowRight`, `AboveRight`).

**Previous approach**: `iced::widget::stack!` with manual backdrops for click-outside-to-close.

**Why changed**: Stack-based overlays couldn't cover the title bar. DropDown uses iced's native overlay system which renders at the window level, handles click-outside automatically, and positions relative to the trigger.

## Window-Level Overlays Composed at App Root

**Decision**: Overlays meant to cover the whole window (bottom sheet, future modals) are composed as siblings of the root column at the App level via `iced::widget::stack`, not nested inside the screen they belong to. Sub-views expose an `Option<Element>`-returning method (e.g. `VaultView::sheet_view(colors, width)`) that App stacks on top of `column![title_bar, page]`.

**Why**: `iced::widget::stack` doesn't cull or clip — every child fully lays out at the same bounds. Stacking a heavy detail subtree inside a content area meant iced walked both the list AND the detail tree on every frame, even when one was visually covered. Hoisting to the App level let the underlying screen take a cheaper exclusive branch (e.g. vault narrow mode renders only the list, no `pane_grid`), so the total per-frame layout work dropped despite the sheet being conceptually "more on top".

**How to apply**: Any future overlay that conceptually covers the entire window — modals, sheets, full-screen confirmations — should expose an `Option<Element>` accessor on the owning view and be stacked at the App level. If the overlay only covers a sub-region, prefer an exclusive branch in the view's own `view()` over stacking.

## Responsive Layout Pattern

**Decision**: For layouts that change based on window size: subscribe to `window::Event::Resized` via `iced::event::listen_with`, store `iced::Size` per `WindowInfo` in `App.windows`, thread the width into the view function, and branch on a `pub const ..._BREAKPOINT_PX: f32` constant near the view module.

**Rationale**: Iced provides no other way to access window size from a view. `view()` re-runs every frame, so the layout swaps automatically on resize without any extra plumbing. Constants keep the breakpoint editable in one place.

**First consumer**: vault detail. `SHEET_BREAKPOINT_PX` in `views/vault/mod.rs` switches between a side `pane_grid` split (wide) and an App-level bottom sheet (narrow).

## Self-Animating Widgets via RedrawRequested

**Decision**: Widgets that need continuous animation (spinner, toast fade) drive their own redraws by intercepting `Event::Window(window::Event::RedrawRequested(now))` in `Widget::update` and calling `shell.request_redraw_at(now + delta)`. No app-level subscription or dummy animation message.

**Prior art**: The toast overlay predates the spinner and established the pattern.

**Rationale**: Subscriptions create every-tick updates that run the whole app `update` cycle even when nothing changed; the `request_redraw_at` approach only redraws the widget tree without running `update`, so the cost stays bounded to the widget that needs the animation.

## Component Library

**Decision**: Button convenience constructors (`buttons::primary(content)`) that take `impl Into<Element>` and return `Button` with style pre-applied. Style functions are internal. Signature aligns with iced's `button()` for familiarity.

**`buttons::ghost` signature**: takes `(content, is_active, active_bg, hover_bg, radius)`. `ghost_icon(content, hover_bg)` is a shorthand for non-active icon-only ghost buttons with `RADIUS_SM`.

## Native Menu Unification

**Decision**: Single `MENUS` static drives both custom and native menus. `NativeMenuHandle` (on App, not static) bridges events.

**Rationale**: Avoids duplicating menu structure. `Shortcut::to_accelerator()` reuses existing `display()` for muda compatibility. `DEV_BOTH_MENUS=1` shows both simultaneously for comparison.

**MenuState shape**: 3 bools (`is_locked`, `has_accounts`, `has_lockable_accounts`). An earlier `has_authenticated_accounts` field was indistinguishable from `has_accounts` in practice — collapsed.

## Widget ID Constants for Cross-File Focus Operations

**Decision**: Widget IDs referenced from more than one file live as `pub const` in the module that owns the widget. Example: `SEARCH_ID: widget::Id = widget::Id::new("vault-search")` in `views/vault/widgets/search_bar.rs`, consumed by `handle_menu_action` for the Cmd/Ctrl-F focus operation.

**Rationale**: String literals repeated across files are a silent-breakage failure mode — one gets updated, the others don't, the focus silently stops working. A single `pub const` forces all callers to import the same identifier.

## SDK Integration: Generator Binary + Per-User SQLite

**Decision**: Build a standalone `tools/fake-data` binary that drives the real Bitwarden SDK (`make_register_keys`, `initialize_user_crypto`, `vault().ciphers().encrypt`, `vault().folders().encrypt`) to populate `<workspace-root>/data/` with one SQLite database per user (filename = user UUID) plus a `mock.json` file holding non-cipher user metadata (email, KDF, encrypted user key, unlock methods). The desktop app discovers users by listing `data/*.sqlite`, pairs each with its `mock.json` entry, and opens the DB via `client.platform().state().initialize_database(Sqlite { db_name, folder_path }, bitwarden_pm::migrations::get_sdk_managed_migrations())`. `ClientManager` then owns one `PasswordManagerClient` per user whose `Cipher`/`Folder` repositories read from SQLite.

**Alternatives considered**: (1) Hand-roll fake `Cipher` structs and skip the SDK — rejected: the app would never exercise real crypto paths. (2) Keep the old single `mock-vault.json` + `MemoryRepo` design — rejected once the SDK grew SQLite support; per-user DBs match production and avoid the 24 MB parse at startup. (3) Embed the DB files in the binary — rejected: `include_bytes!`ing SQLite files bloats the binary and forbids writes.

**Rationale**: Unlock is a real `crypto().initialize_user_crypto(...)` call against real encrypted data; `list_ciphers` is a real `decrypt_list` round-trip. Data lives in real SDK-managed storage, not a hand-rolled `MemoryRepo`, so any future change to the SDK's persistence layer is automatically exercised in dev.

**Caveats**: `LocalUserDataKeyState` isn't in `get_sdk_managed_migrations()` but `initialize_user_crypto` writes to it, so a narrow `MemoryRepo<LocalUserDataKeyState>` is registered client-managed per user. We also hand-assemble the `PasswordManagerClient` via `ClientBuilder` with `StateRegistry::new()` because `PasswordManagerClient::new` defaults to `new_with_memory_db`, which pre-sets the database `OnceLock` and blocks our `initialize_database` call. Both sites have a TODO to switch to `PasswordManagerClient::load_from_state` once the SDK exposes it.

**Dev passwords** (documented in `sdk.rs`): `alice@example.com` / `password`, `alice@acmecorp.com` / `123456`, `loadtest@example.com` / `loadtest` (20 k ciphers).

**Shared schema synchronization**: `MockVaultFile` / `MockUser` / `UnlockMethodsCfg` structs are defined independently in `crates/desktop/src/sdk.rs` and `tools/fake-data/src/main.rs`, with comments telling the reader to keep the field sets in sync. A shared schema crate (`tools/mock-schema`) is a future refactor — current synchronization is manual but easy to spot in review.

## Cipher UI Types: SDK Directly

**Decision**: The vault view, item list, and detail pane consume `bitwarden_vault::CipherListView` and `CipherView` directly. No intermediate `state::CipherItem` DTO. `VaultView.items.cached` is `Vec<Arc<CipherListView>>`.

**Rationale**: Once the SDK pipeline is in place, keeping a mirror DTO is pure overhead — every new SDK field would need duplicating.

**`Arc<CipherListView>` wrapping**: `CipherListView` doesn't derive `Clone`, and iced `Message`s must be `Clone`. Wrapping in `Arc` gives cheap clones for message dispatch and filter recomputation. `CipherView` is smaller and does derive `Clone`, so it's passed as `Box<CipherView>` in messages and stored as `Option<CipherView>` on `VaultView`.

**Stale-load guard**: When the user clicks item A, then item B before A's decrypt completes, both `DetailLoaded` messages arrive but only B should apply. The guard is `self.selection.id == view.id` in `VaultView::update` for the `DetailLoaded` arm.

## Toast Notifications: Custom Widget + Unified Cells

**Decision**: `components::toast::Manager` is a custom `Widget` that wraps the app's main content and overlays a vertical toast stack via iced's native `Overlay` trait. Animation state (fade in/out, countdown progress, hover-pause) lives in two places:

1. **Persistent timer state** in the widget tree (`Vec<Option<ToastTimer>>`): creation timestamp, optional dismissal-start timestamp, hovered bool. Survives across `view()` rebuilds because iced's tree state outlives `Manager` instances.
2. **Per-frame animation values** in `Vec<Rc<Cell<ToastVisuals>>>` on `Manager`, paired 1:1 with the toast list. `ToastVisuals { alpha, progress }` is `Copy`. Refreshed on every `window::Event::RedrawRequested` tick from the timer state.

Style closures on the row widgets capture an `Rc<Cell<ToastVisuals>>` at construction time and read it at draw time. Updating the cell from inside the overlay's `update()` drives a re-render without layout invalidation.

**Auto-dismiss is two-phase**: timer expires → `dismissing = Some(now)` → fade-out runs over 150ms → once complete, `on_close(idx)` is published and App removes the toast. Manual close bypasses the fade. Hovering pins `alpha = MAX_ALPHA` + `progress = 1.0`.

**Progress bar rounding**: the fill quad's bottom-left corner rounds to `RADIUS_MD` (capped to half the fill width for narrow slivers), bottom-right rounds only when the fill reaches the toast's right edge — so the fill follows the toast's rounded corners without hard-clipping. The fill is inset 1 px on left/right/bottom so it floats inside the corners.

## Error Messages in Toasts: Sanitize SDK Output

**Decision**: Don't echo `e.to_string()` from SDK errors directly into toast bodies. Log the raw error at `warn!`/`error!`, show the user a short sanitized message.

**Rationale**: SDK `Display` impls can surface internal crypto state, key-derivation iteration counts, or module paths in error text — not appropriate for a UI toast. Users see "Check your master password and try again" for unlock failures; the raw SDK error lives in the tracing log for developers.

## `#[expect(dead_code)]` Over `#[allow(dead_code)]`

**Decision**: Use `#[expect(lint)]` rather than `#[allow(lint)]` for deliberate suppressions. The `expect` form emits a warning when the suppression becomes unnecessary, catching stale allowances automatically.

**Exception**: `components/icons.rs` uses `#![allow(dead_code)]` at file scope because the `bootstrap_icons_generated.rs` include generates hundreds of `Icon` constants, many unused. `expect` at file scope would not fire correctly since *some* constants are used. Alternative: `#[expect(dead_code)]` on each unused constant is not feasible in generated code.

## Workspace-Level Shared Dependencies

**Decision**: Dependencies used by more than one workspace member (`async-trait`, `serde`, `serde_json`, `tokio`, `tracing`, `tracing-subscriber`, `uuid`, and all five `bitwarden-*` git-pinned crates) are declared once under `[workspace.dependencies]` in the root `Cargo.toml` and consumed as `{ workspace = true }`.

**Rationale**: The `bitwarden-*` git revision is especially important to keep in lockstep — if `tools/fake-data` encrypts with one revision and `crates/desktop` decrypts with another, the JSON won't round-trip.

**Crate-local deps stay local**: iced, iced_aw, image, muda, system-theme (desktop only), chrono (fake-data only), cargo-packager (packager only).

## Packaging: cargo-packager

**Decision**: Use `cargo-packager` for distribution packaging (.app, .dmg, .msi). Configuration in `Packager.toml` at workspace root. A thin wrapper crate at `tools/packager/` invokes `cargo_packager::cli::run`.

**Previous approach**: `winresource` build script for Windows exe icon, `embed_plist` for macOS bundle metadata.

**Why changed**: `cargo-packager` handles all platform packaging concerns in one place — bundle structure, installer creation, icon embedding, code signing — removing the per-platform build-script hacks.

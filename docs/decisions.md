# Decisions

## Framework: Iced 0.14

**Decision**: Use Iced as the GUI framework.

**Alternatives considered**: Dioxus (webview-based desktop mode, custom renderer still maturing), Slint (GPL/commercial license concern).

**Rationale**: Pure Rust, wgpu rendering, no webview, Elm architecture ensures repaints only on state change, ~25k GitHub stars, actively maintained, cross-platform.

## Crate Name: bitwarden-desktop-next

**Decision**: Named `bitwarden-desktop-next` to avoid conflict with existing "bitwarden-lite" project.

## Workspace Structure

**Decision**: Organize as a Cargo workspace with the desktop app at `crates/desktop/`, plus `bitwarden_license/*` and `tools/*` as additional members. Only `crates/desktop` is in `default-members`.

**Rationale**: Mirrors the pattern used in other Bitwarden Rust projects. Separates the main application from licensed code and developer tooling. `default-members` ensures `cargo run` / `cargo build` target only the desktop app by default.

## UI Only (No Business Logic)

**Decision**: This app is stub UI only. All crypto, API, authentication, and vault operations will come from a separate SDK (`PasswordManagerClient`).

**Rationale**: Separation of concerns. The SDK handles security-sensitive operations; this project handles presentation.

## Multi-User State Model

**Decision**: App state uses `HashMap<UserId, UserSession>` mirroring the SDK's `HashMap<UserId, PasswordManagerClient>`.

**Rationale**: Makes future SDK integration a clean swap. Multiple accounts can be available simultaneously with one active at a time.

## Colors: Picker Over SCSS

**Decision**: Use color-picked values from the actual running Bitwarden app, not SCSS/CSS variable values.

**Rationale**: The SCSS values in `variables.scss` and Tailwind CSS vars don't match what the app actually renders. For example, `backgroundColor` is `#1f242e` in SCSS but the actual window background is `#070b18`. The SCSS has three background levels (`backgroundColor`, `backgroundColorAlt`, `backgroundColorAlt2`) and it's unclear which applies where. Always verify with a color picker (Win+Shift+C with PowerToys).

## Menus: Native on macOS, Custom-Drawn Elsewhere

**Decision**: Use `muda` for native menus on macOS only. Draw the menu bar inside iced on Windows and Linux.

**Previous approach**: Used `muda::init_for_hwnd()` on Windows for native Win32 menus.

**Why changed**: Native Win32 menus create a 1px transparent gap between the menu bar and the iced client area (DWM compositor border). This is a known muda issue (#123, fixed for Tauri's webview but not for raw wgpu rendering). The dark theme (`MenuTheme::Dark`) made the entire menu bar transparent instead of dark. Drawing the menu ourselves eliminates the gap entirely.

**macOS**: Keeps native menus via `init_for_nsapp()` because macOS users expect the system menu bar at the top of the screen, and it integrates properly there.

**Linux**: Uses the custom-drawn menu (same as Windows) to avoid GTK dependencies.

## Icons: Bootstrap Icons (Self-Bundled)

**Decision**: Bundle Bootstrap Icons TTF font directly and auto-generate Rust constants via `build.rs`.

**Alternatives considered**: `iced_fonts` crate (adds a dependency for something trivial), `iced_aw` icons (claimed to have an Icon enum but doesn't), Font Awesome, Nerd Fonts, individual SVGs per icon.

**Rationale**: Bootstrap Icons is what the official Bitwarden app uses (`bwi-*` prefixed classes). The crates that wrap icon fonts are trivially simple — they just bundle a TTF and map names to Unicode codepoints. Doing it ourselves means zero extra dependencies and full control over the version. The `build.rs` script parses the official CSS file and generates an `Icon` type with `.render()` method.

**WOFF→TTF conversion**: The Bootstrap Icons release only ships WOFF/WOFF2, but iced's `fontdb` only supports TTF/OTF. We used the `wuff` crate (pure Rust WOFF decoder) once to convert the WOFF to TTF, saved the result in `assets/`, then removed `wuff` from dependencies.

## Font: Inter 18pt Static (Medium/Bold)

**Decision**: Bundle static Inter 18pt font files (`Inter_18pt-Medium.ttf` + `Inter_18pt-Bold.ttf`) with default weight Medium (500). Family name: `"Inter 18pt"`. Constants `APP_FONT` / `APP_FONT_BOLD` in `main.rs`.

**Previous approach**: Inter variable font (`InterVariable.ttf`) at Normal (400) weight.

**Why changed**: The "18pt" optical size variant has an open lowercase "g" (single-storey) which better matches the official Bitwarden app rendering. The standard Inter variable font uses a closed/double-storey "g" that looked different. Medium (500) weight also better matches the app's visual density. Static font files were chosen over variable to avoid the optical size axis complexity.

## Font Sizes: Consolidated to 5

**Decision**: Use exactly 5 font sizes: 12, 14, 16, 18, 28. Down from 10 distinct sizes.

**Rationale**: Fewer sizes create a more consistent visual hierarchy. These 5 values cover all current needs: 12 for captions, 14 for body/buttons, 16 for emphasis, 18 for section headers, 28 for page titles.

## Button Style: Pill-Shaped (20px radius)

**Decision**: Use 20px border-radius for buttons (more pill-shaped than the 12px from the component library).

**Rationale**: Visual inspection of the actual rendered app shows more rounded buttons than the Tailwind `tw-rounded-xl` (12px) suggests. 20px matches the actual look better.

## Overlays: Legacy stack! Approach (Replaced)

**Previous decision**: Used `iced::widget::stack!` at the page level for floating elements.

**Status**: Replaced by DropDown widget (see "Overlays: DropDown Widget" section below). Stack approach had limitations: couldn't cover the title bar, required manual backdrops, fragile offset calculations.

## Window Title

**Decision**: Use "Bitwarden [Next]" as the window title.

**Rationale**: Avoids conflict with the real Bitwarden app when running side-by-side for visual comparison. The PowerShell screenshot script used `FindWindow` by title, which would match the wrong window otherwise.

## Theme System (Implemented)

**Decision**: Custom `AppTheme` struct implementing `iced::theme::Base` + widget `Catalog` traits. Dark/light palettes in separate files.

**Status**: Complete. Light theme is the default. Runtime toggle via Help > About Bitwarden. Light palette colors picked from design mockup (`designs/Desktop 2025/vault-view item.png`).

**Rationale**: iced 0.14's `application()` is generic over Theme. Custom theme gives `&AppTheme` in all `.style()` closures with direct access to 20+ semantic color tokens (including `nav_text` and `nav_item_hover` for sidebar-specific colors). No thread_local or registry needed.

## Overlays: DropDown Widget (replaces stack! pattern)

**Decision**: Use a local fork of iced_aw's `DropDown` widget (`components/drop_down.rs`) with custom alignment variants (`BelowLeft`, `BelowRight`, `AboveRight`).

**Previous approach**: `iced::widget::stack!` with manual backdrops for click-outside-to-close.

**Why changed**: Stack-based overlays couldn't cover the title bar (they're inside the page layout). DropDown uses iced's native overlay system which renders at the window level, handles click-outside automatically, and positions relative to the trigger widget.

## View Architecture: Compositional MVU

**Decision**: Each view (`LoginView`, `VaultView`, `TitleBarState`) owns its local state and async work, and returns `(Task<SubMessage>, Option<Event>)` from its `update()` method. The parent `App` is a thin router that lifts sub-tasks via `Task::map(Message::Sub)` and translates events into cross-cutting side effects.

**Status**: Implemented. Replaces the previous `Vec<Action>` pattern.

**Previous approach**: Sub-views returned `Vec<LoginAction>` / `Vec<VaultAction>` / `Vec<TitleBarAction>` from `update(msg) -> Vec<_>`. Actions were imperative ("App please call `mgr.unlock(pw)` now") and App owned every `Task::perform` call. Async completions lived as top-level `Message::UnlockCompleted` / `Message::VaultListLoaded` / `Message::CipherDetailLoaded` variants because `Task::perform` returns `Task<Message>` and nothing forced those callbacks to re-enter the view that cared about them.

**Why changed**: the top-level `Message` enum had grown to 13 variants — three sub-view wrappers plus 10 orphans — and `App::update()` had grown to 220 lines mixing screen routing, async dispatch, toast pushing, menu handling, theme sync, and window chrome. Every new feature required touching `Message`, `update()`, and usually a new App field. `app.rs` was the single shared-write bottleneck and would have blocked multi-team development of the upcoming production rewrite this project is a testbed for.

**New pattern**:

1. **Sub-views own their async work.** `LoginView::update(LoginMessage::Unlock, &client_manager, active_user)` calls `Task::perform(mgr.unlock(...))` directly, returning `Task<LoginMessage>`. The parent router lifts it with `.map(Message::Login)`.
2. **Async callbacks re-enter the owning view.** `LoginMessage::UnlockCompleted(uid, result)` is a variant of `LoginMessage`, not the top-level `Message`. The view handles the completion, performs a stale-check against `active_user`, and emits a completed-state event (`LoginEvent::Unlocked { uid }`) for App to route.
3. **Events are declarative.** `LoginEvent::Unlocked { uid }` means "the user has unlocked" — a domain fact. `handle_login_event` in App translates that fact into a screen switch and kicks the follow-on `load_vault_list_task`.
4. **`ClientManager` is injected call-time**, passed as `&Arc<ClientManager>` to each view's `update()`. Views remain cheaply constructible in tests without a real SDK instance.
5. **Cross-cutting dismissal stays at the router level.** The pre-match block at the top of `App::update` closes the other view's overlays on every sub-view message, so sub-views don't need to know about each other.

**Prior art**:
- **Halloy** — [investigation/halloy/src/buffer.rs:247](../investigation/halloy/src/buffer.rs) — `buffer.update()` signature is `(Task<Message>, Option<Event>)`, lifted at call sites via `command.map(Message::Dashboard)`.
- **cosmic-settings** — [investigation/cosmic-settings/cosmic-settings/src/pages/mod.rs:113](../investigation/cosmic-settings/cosmic-settings/src/pages/mod.rs) — `impl From<pages::Message> for crate::Message` so pages construct local messages and the lift is transparent.
- **iced-guide** — [investigation/iced-guide/src/app_structure/composition.md](../investigation/iced-guide/src/app_structure/composition.md) — canonical "composition pattern" matching Halloy's shape exactly.

**Result**: Top-level `Message` enum shrinks from 13 → 5 (`Login`, `Vault`, `TitleBar`, `Window`, `System`). `App::update()` body drops from 220 → ~60 lines (pure router, plus a pre-match cross-view dismissal block and a `post_update()` call). Adding a new async operation inside a view now touches only that view's file; adding a whole new screen touches the view directory plus ~4 mechanical lines in `app.rs` (enum variant, struct field, router arm, event handler). See [docs/architecture.md](architecture.md) "How to Add a New View" for the recipe.

**Daemon-ready shapes**: `WindowMessage` (per-window OS events) and `SystemMessage` (global signals) are split even though single-window today. Every `WindowMessage` variant carries `window::Id`. When we migrate to `iced::daemon` for multi-window support, the message routing doesn't change — `main.rs` swaps `iced::application(...)` for `iced::daemon(...)` and the sub-view dispatch stays exactly as it is.

**Action → Event semantic rename**: Imperative Actions became declarative Events during the refactor. `LoginAction::Unlock(pw)` (request) → the view running `Task::perform` directly + `LoginEvent::Unlocked { uid }` (completed fact). `LoginAction::LoadDetail(id)` → view owns `Task::perform(mgr.full_cipher(...))` and emits no event (detail cache updates internally). This shift is load-bearing: it's what lets async lifecycles live with the view that cares about them.

**Consequences**:
- Can't easily go back — the `Vec<Action>` pattern is gone from all three views at once. Reverting requires undoing the entire PR.
- New contributors learn the pattern once per sub-view they work on. The "How to Add a New View" recipe in `docs/architecture.md` is the on-ramp.
- `app.rs` should drift toward being the smallest, most stable file in the crate. Commits that grow `app.rs` for feature work are a smell; they should grow view files instead.
- Toast emission from sub-views routes through events (`LoginEvent::ToastRequested(Toast)`), keeping the toast queue on App.
- Stale-checks (for in-flight tasks when the user switches accounts or clicks a different item) co-locate with the view that owns the state they protect, not with App.

**Kill-switch criteria**: revisit this pattern if (a) sub-views need to nest multiple levels deep (a sub-view containing another sub-view with its own async work) — the `Task::map` chain becomes awkward past two levels, or (b) we end up needing a message router that runs before the sub-view update (e.g. undo/redo middleware) — the one-match router would need to grow into something bigger.

## Component Library

**Decision**: Button convenience constructors (`buttons::primary(content)`) that take `impl Into<Element>` and return `Button` with style pre-applied. Style functions are internal.

**Rationale**: Callers never need to write `.style()` closures for common button variants. The `impl Into<Element>` signature matches iced's `button()` for familiarity.

## Native Menu Unification

**Decision**: Single `MENUS` definition drives both custom and native menus. `NativeMenuHandle` (on App, not static) bridges events.

**Rationale**: Avoids duplicating menu structure. `Shortcut::to_accelerator()` reuses existing `display()` for muda compatibility. `DEV_BOTH_MENUS=1` shows both simultaneously for comparison.

## SDK Integration: Generator Binary + Embedded JSON

**Decision**: Build a standalone `tools/fake-data` binary that drives the real Bitwarden SDK (`make_register_keys`, `initialize_user_crypto`, `vault().ciphers().encrypt`, `vault().folders().encrypt`) to produce `assets/mock-vault.json`, then embed that JSON in the desktop crate via `include_bytes!`. The desktop app parses it at startup into `ClientManager`, which owns one `PasswordManagerClient` per user and pre-populates each client's `MemoryRepo<Cipher>` / `MemoryRepo<Folder>` with the encrypted data.

**Alternatives considered**: (1) Hand-roll fake `Cipher` structs in Rust and skip the SDK entirely — rejected because it means the app never exercises real encrypt/decrypt code paths. (2) Generate a `.rs` file with `pub fn users() -> Vec<MockUser>` instead of JSON — rejected because `Cipher`/`EncString`/`CipherId`/`DateTime` aren't `const`-constructible, so the generated file would just be `parse::<EncString>().unwrap()` and `vec![...]` calls (i.e. serde deserialization in disguise), with the extra burden of emitting valid Rust and tracking SDK field churn. (3) Load the JSON at runtime from disk — rejected because `include_bytes!` is simpler and we don't need live reload (regeneration is a `cargo run -p fake-data` away).

**Rationale**: `make_register_keys` + `initialize_user_crypto` + `vault().ciphers().encrypt(...)` exercises the exact code paths that a real client would hit during sign-up. The embedded JSON means the desktop app has a guaranteed-consistent mock vault for every build — no external service required, no network calls, no fixture sync problems. Unlock in the app is a real `crypto().initialize_user_crypto(...)` call against real encrypted data; `list_ciphers` is a real `decrypt_list` round-trip. Tests in `sdk.rs` cover the unlock → list → decrypt chain end-to-end with no mocking.

**Dev passwords** (documented in `sdk.rs`): `alice@example.com` / `password`, `alice@acmecorp.com` / `123456`.

## Cipher UI Types: SDK Directly, No Intermediate DTO

**Decision**: The vault view, item list, and detail pane consume `bitwarden_vault::CipherListView` and `CipherView` directly. There's no intermediate `state::CipherItem` struct. `VaultView.items.cached` is `Vec<Arc<CipherListView>>`; the detail pane branches on `CipherView.r#type` and renders per-type field cards.

**Previous approach**: Had a flat `state::CipherItem { id, name, username, url, category }` that mirrored the SDK shape loosely, with a hand-written `mock::mock_vault_items_for(uid)` as the data source. Made it easier to iterate on the UI without wiring the SDK, but meant two sources of truth and a translation layer.

**Why changed**: Once the SDK pipeline was in place (see above), keeping `CipherItem` was pure overhead. Every new field on `CipherListView` would need to be duplicated in `CipherItem`, and the detail pane couldn't render Card/Identity/SshKey details without making up new mirror types. Going straight to SDK types removed the entire `mock.rs` module.

**`Arc<CipherListView>` wrapping**: `CipherListView` doesn't derive `Clone`, and iced `Message`s must be `Clone`. Wrapping in `Arc` gives cheap clones for both message dispatch and filter recomputation. `CipherView` is smaller and does derive `Clone`, so it's passed as `Box<CipherView>` in messages (keeps the enum size down) and stored as `Option<CipherView>` on `VaultView`.

**Stale-load guard**: When the user clicks item A, then item B before A's decrypt completes, both `CipherDetailLoaded` messages arrive but only B should apply. The guard is two lines in `VaultView::set_selected_detail`: `if self.selected_id == view.id`. Otherwise a fast A → B click could clobber B's view with A's.

## Toast Notifications: Custom Widget + Unified Cells

**Decision**: `components::toast::Manager` is a custom `Widget` that wraps the app's main content and overlays a vertical toast stack via iced's native `Overlay` trait. Animation state (fade in/out, countdown progress, hover-pause) lives in two places:

1. **Persistent timer state** in the widget tree (`Vec<Option<ToastTimer>>`): creation timestamp, optional dismissal-start timestamp, hovered bool. Survives across `view()` rebuilds because iced's tree state outlives `Manager` instances.
2. **Per-frame animation values** in `Vec<Rc<Cell<ToastVisuals>>>` on `Manager`, paired 1:1 with the toast list. `ToastVisuals { alpha, progress }` is `Copy`. Refreshed on every `window::Event::RedrawRequested` tick from the timer state.

Style closures on the row widgets capture an `Rc<Cell<ToastVisuals>>` at construction time and read it at draw time. Updating the cell from inside the overlay's `update()` drives a re-render without any layout invalidation.

**Alternatives considered**: (1) Iced's official toast example verbatim — the starting point, but it uses `container::primary/success/...` theme functions that don't map to our custom `AppTheme`. (2) `iced-toasts` crate — small, pulls in no meaningful abstraction, simpler to own the ~600 lines in-crate. (3) Spawning a separate window for each toast — rejected, toasts are ephemeral and window lifecycle is overkill.

**Why custom cells instead of rebuilding Elements per frame**: Toast row `Element`s are built once per `view()` call inside `Manager::new`. The overlay's `update()` runs per-frame but can't reassign `self.toasts` because of the borrow graph (`&'b mut self.toasts` is held by the overlay::Element it returns). Cells are the simplest bridge: one `Rc<Cell<_>>` per toast, shared between the overlay (writer) and the style closures (readers).

**Auto-dismiss is two-phase**: timer expires → `dismissing = Some(now)` → fade-out runs over 150ms → once complete, `on_close(idx)` is published and App removes the toast from its `Vec`. Manual close (clicking ×) bypasses the fade. Hovering pins `alpha = MAX_ALPHA` and `progress = 1.0` and clears any in-flight dismissal; moving the cursor out resumes countdown from full.

## Workspace-Level Shared Dependencies

**Decision**: Dependencies used by more than one workspace member (`async-trait`, `serde`, `serde_json`, `tokio`, and all five `bitwarden-*` git-pinned crates) are declared once under `[workspace.dependencies]` in the root `Cargo.toml` and consumed as `{ workspace = true }` from the member `Cargo.toml` files.

**Rationale**: The `bitwarden-*` git revision is especially important to keep in lockstep — if `tools/fake-data` encrypts with one revision and `crates/desktop` decrypts with another, the JSON won't round-trip. Shared declaration guarantees both sides compile against the exact same SDK. Same logic for `serde_json` — if the generator serializes with one version and the desktop app deserializes with another, we'd be one breaking change away from a silent mismatch.

**Crate-local deps stay local**: iced, iced_aw, image, muda, system-theme (desktop only), chrono, uuid (fake-data only), cargo-packager (packager only). No point promoting them to the workspace level.

## Packaging: cargo-packager

**Decision**: Use `cargo-packager` for distribution packaging (.app, .dmg, .msi). Configuration in `Packager.toml` at workspace root. A thin wrapper crate at `tools/packager/` invokes `cargo_packager::cli::run`.

**Previous approach**: `winresource` build script for Windows exe icon, `embed_plist` for macOS bundle metadata.

**Why changed**: `cargo-packager` handles all platform packaging concerns in one place: .app bundle structure, .dmg creation, .msi installer, icon embedding, code signing. The per-platform build script hacks (`winresource`, `embed_plist`) were removed since `cargo-packager` handles these natively.

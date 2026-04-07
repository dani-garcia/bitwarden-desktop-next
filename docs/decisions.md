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

## State Decentralization (Implemented)

**Decision**: Each view (LoginView, VaultView, TitleBarState) owns its state and has its own `update()` method returning action enums.

**Status**: Complete. App struct down from 30 to 11 fields.

**Rationale**: Different views will be owned by different teams. Event handling should be close to the components that initiate events. Action enums handle cross-cutting effects cleanly.

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

**Decision**: The vault view, item list, and detail pane consume `bitwarden_vault::CipherListView` and `CipherView` directly. There's no intermediate `state::CipherItem` struct. `VaultView::cached_items` is `Vec<Arc<CipherListView>>`; the detail pane branches on `CipherView.r#type` and renders per-type field cards.

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

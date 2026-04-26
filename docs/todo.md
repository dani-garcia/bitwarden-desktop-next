# TODO

Tasks are organized into four rough tiers by priority. Within a tier, pick what's
most unblocking or most interesting — the tiers are not a strict ordering. Completed
work isn't tracked here; check `git log` or [docs/decisions.md](./decisions.md) for context.

**Keep this file in sync.** When you finish (or partially finish) a task, update or
remove its entry in the same change — don't leave it for later. Reduced scope is fine
(shrink the entry to what's still left) as long as the remaining work is still correct.

## Contents

- [Tier 1 — Do Next](#tier-1--do-next)
- [Tier 2 — User-Visible Features](#tier-2--user-visible-features)
- [Tier 3 — Research & Architecture](#tier-3--research--architecture)
- [Tier 4 — Deferred / Waiting Upstream](#tier-4--deferred--waiting-upstream)

---

## Tier 1 — Do Next

These are the highest-value tasks with no architectural prerequisites. Roughly ordered by importance × ease — quick-wins first, research tasks last.

### Implement the login command

The login-email flow in [crates/desktop/src/views/login/login_email.rs](../crates/desktop/src/views/login/login_email.rs) presently stubs after `ContinueWithEmail` — no network call, no session creation. Wire it to the real SDK login path so a user with an email + master password can actually authenticate (not just unlock an existing SQLite DB populated by `fake-data`).

- Plumb `bitwarden_auth`'s password-login API through `ClientManager`.
- Handle CAPTCHA / device-verification responses (surface as toast + retry).
- Error sanitisation per the toast rule above.
- The 2FA screens that gate many real accounts are out of scope here — tracked separately under Tier 2 "Auth flow completion".

### Study `desktop_native` for SSH agent + biometrics (plan mode)

Before wiring the many SDK/OS-backed stubs below, produce a written plan for whether and how to pull in the `desktop_native` core modules from `clients/apps/desktop/desktop_native/`. Scoped under the "Integrate `desktop_native`" entry in Tier 3 — see that section for the full investigation questions. This Tier 1 slot is just *doing* the study; the implementation follows.

### Wire up the stubbed settings

All 24 settings in the Appearance / Security / Integrations / Autotype / Advanced tabs now persist to `data/settings.json`, but most are UI-only stubs that emit `settings-toast-not-supported` when toggled. Grouped roughly by blocker:

- **Zero SDK work — OS/wiring glue**
  - Open at device login — write to platform autostart (registry on Windows, LaunchAgent on macOS, `.desktop` file on Linux).
  - Minimize on copy — flip a flag read by `ClipboardManager::copy` call-sites.
  - Always show dock (macOS) — `NSApplication.setActivationPolicy`.
  - Enable hardware acceleration — Help → Troubleshooting → Toggle hardware acceleration now flips `Settings::hardware_acceleration`, persists, and toasts that a restart is required. The Advanced settings-tab checkbox still saves silently — give it the same restart toast. Also note that the `gpu` cargo feature must be built in for the persisted flag to do anything (`main::select_backend` falls back to tiny-skia when the feature is off).
  - Allow screenshots — Windows `SetWindowDisplayAffinity`; macOS / Linux set at window creation, so also restart-required.
  - Show favicons — list renders real favicons via the lazy service in
    [crates/desktop/src/favicon.rs](../crates/desktop/src/favicon.rs).
    Remaining:
    - Wire the same `favicon.get(uid, host)` lookup into the detail pane
      (it still renders the initial-letter circle).
    - **Per-user `icons_url` for self-hosted.** Today all users resolve to
      `https://icons.bitwarden.net`. Add `icons_url: String` to `UserEntry`
      in [sdk.rs](../crates/desktop/src/sdk.rs), populated from
      `/api/config`'s `environment.icons` at login time (cloud defaults:
      US → `icons.bitwarden.net`, EU → `icons.bitwarden.eu`, per the
      Angular clients' `default-environment.service.ts`). Expose
      `ClientManager::icons_url(&uid) -> Option<String>` and swap the
      resolver closure in [app/mod.rs](../crates/desktop/src/app/mod.rs)
      to consult it (fallback to US cloud default when `None`). Closure
      runs per fetch so re-auth against a different server picks up the
      new URL without restart. Blocked on the login command landing (or
      the self-hosted URL modal in Tier 2) so a real `icons_url` exists.

- **SDK integration**
  - PIN unlock — per-user PIN state via `bitwarden_auth` + keystore wrapping.
  - Session timeout (Lock after / Log out after) — background timer driven by `iced::time::every`, locks the active user on expiry.
  - Browser integration (+ fingerprint) — native-messaging host registration.
  - DuckDuckGo browser integration (macOS only).
  - Enable autotype (Windows Premium) — the autotype engine itself is a whole feature.

- **Depends on `desktop_native` study above**
  - Touch ID / Windows Hello / polkit biometrics unlock.
  - *(SSH agent moved to Tier 4 — waiting on upstream V2.)*

---

## Tier 2 — User-Visible Features

Features that complete the happy paths users expect. No architectural work required — each is a contained feature addition following the "how to add a new view" recipe.

### Auth flow completion

The login command itself is promoted to Tier 1 — these are the screens / flows that surround it.

- **Two-factor authentication screens** — after a successful `ContinueWithEmail` the server may demand TOTP / Duo / WebAuthn / email code. Each is its own `AuthPage` variant under `LoginView`. Not required to land Tier 1's login command, but any account with 2FA enabled can't actually finish logging in until these exist.
- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields. New sub-view under `views/register/` plus a new `Register` screen variant or inlined into `LoginView`.
- **Master password hint request** — "Get master password hint" link on login password screen sends a hint request to the server.
- **Self-hosted server URL modal** — server selector's "Self-hosted" option should open a modal to input custom server URL.
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

### Tray icon — Linux packaging

The tray itself ships as of this change (see `crates/desktop/src/tray.rs`, four tray-adjacent settings in `data/settings.json`, single-instance wake-up). Residual Linux work:

- Add `libayatana-appindicator3-1` as a runtime dep in the `.deb` / `.rpm` produced by `cargo run --bin packager`.
- Note the requirement in the Linux install README (without it or an SNI host, `tray::build()` returns `None` and tray settings become no-ops; this is intentional but should be documented).
- Verify GNOME + AppIndicator extension, KDE Plasma, and sway + waybar behaviour on a test VM.

**Note:** iced [PR #3021](https://github.com/iced-rs/iced/pull/3021) adds native tray support but is still open (targeting 1.0). If it lands on our pin, re-evaluate swapping our `tray-icon` dep for the native path.

### Right-click context menu for text inputs

Standard cut / copy / paste / select-all on `TextInput` fields and the notes `TextEditor` in `cipher_form`. Iced 0.15 doesn't provide this out of the box — every text widget silently swallows right-clicks. Shape: a `components::context_menu` wrapper that stacks `MouseArea::on_right_press` over the child and shows our `DropDown` with the four actions. `TextEditor` already takes `Action::{Copy,Cut,Paste,SelectAll}` via its `on_action` callback, so the notes field is a direct wire-up. `TextInput` needs a `widget::Id` per field + `widget::operation::text_input::{select_all, ...}` dispatched as `Task`s; paste reuses iced's clipboard shell. Simplest first pass anchors the menu to the field (our `DropDown` is widget-anchored, not cursor-anchored); a cursor-anchored variant would need a small `DropDown` extension for absolute-offset placement.

### Sidebar polish

- **Indented tree hierarchy** — Vault > All vaults > My vault, with visual indent.
- **Expand/collapse animation** — sidebar width snaps between `RAIL_WIDTH` and `PANEL_WIDTH`. Wire a `lilt::Animated<f32, Instant>` for the width and call `services::animation::extend(...)` on toggle so the App's frame subscription picks it up automatically (same pattern as the generator's segmented-pill swoosh — see [decisions.md](./decisions.md) → "State-Driven Animation").
- **Active-row crossfade** — animate the bg color of the selected row in/out instead of snap. Cheap visual polish via `lilt::Animated<bool>` per row, or a single `Animated<usize>` that picks which row paints accent.

### Send — wire to real SDK + supporting flows

Sends currently live as decrypted `SendView`s in an in-memory `HashMap` on [`ClientManager`](../crates/desktop/src/services/sdk/mod.rs) (see `list_sends` / `full_send` / `save_send` / `delete_send`). Empty at startup; mutations never leave the process.

- **SDK repository swap.** Replace the in-memory map with the SDK's `Repository<Send>` + `SendClient::{encrypt, decrypt, decrypt_list}`. The method signatures on `ClientManager` are already async + fallible so call sites don't need to change. Remove the `uuid::Uuid::new_v4()` id fabrication in `save_send` once the SDK assigns the id; the placeholder `access_id` built there should come from `CreateSendResponse` instead.
- **Password regenerate button.** The refresh icon on the send form's password field currently calls a local 14-char alphanumeric generator (`SendForm::regenerate_password` in [widgets/send_form/state.rs](../crates/desktop/src/views/send/widgets/send_form/state.rs)). Swap for the real `ClientManager::generate_password` (the Generator modal now uses this via `bitwarden-generators`) — the existing user-facing options on that generator should drive the Send form's field too.
- **File Send creation.** The "Choose file" button in the new-file-send branch of the form ([widgets/send_form/view.rs](../crates/desktop/src/views/send/widgets/send_form/view.rs) `file_section`) is a placeholder — message fires, handler is a no-op. Needs an OS file picker (dialog crate or iced's native picker once it lands) to populate `file_name` + `file_size_name`, plus the `SendClient::encrypt_file` / `encrypt_buffer` wiring.
- **Real send link.** `SendForm::send_link` in [state.rs](../crates/desktop/src/views/send/widgets/send_form/state.rs) builds a stub URL (`http://vault.bitwarden.test/#/send/{access_id}`). Once the SDK's create path returns a real `access_id` + key fragment the formatting rule moves to using the user's configured `server_url` and the actual URL shape.

### Magnify launcher polish

The V1 launcher ships behind `Ctrl+Shift+Space` ([crates/desktop/src/views/magnify/](../crates/desktop/src/views/magnify) + [services/global_hotkey/](../crates/desktop/src/services/global_hotkey/mod.rs)). Follow-ups, in roughly increasing scope:

#### Settings surface

- **Master enable/disable toggle.** No way to turn Magnify off today — the global hotkey registers unconditionally at startup. Add `Settings::magnify_enabled: bool` (default `true`), gate `services::global_hotkey::install_event_handler` on it, and expose a switch in the Integrations / Autotype settings tab. Toggling at runtime should re-register or unregister the hotkey without restart (call `GlobalHotKeyManager::unregister` on disable, re-call `install_event_handler` on enable — needs the `OnceLock<Sender>` to outlive the disable so re-enable doesn't re-leak a second manager).
- **Settings UI for the hotkey.** V1 hardcodes `Ctrl+Shift+Space` (`Cmd+Shift+Space` on macOS). Add a binder in the same settings panel; persist to `Settings::magnify_hotkey` as a parsed `(Modifiers, Code)` pair. Re-register on change via a fresh `GlobalHotKeyManager::register`. The `"Ctrl+C"` / `"Ctrl+⇧C"` strings hardcoded in [chip helpers](../crates/desktop/src/views/magnify/widgets/chip.rs) and [row action pills](../crates/desktop/src/views/magnify/widgets/row.rs) should also drive off the bound config so the displayed shortcut matches the actual binding.

#### Behaviour / UX

- **Cursor-monitor centering on summon.** Today the launcher centers on the primary monitor each summon. iced 0.15 doesn't expose `winit::MonitorHandle`, so this needs platform-specific shims. The service scaffold ships at [crates/desktop/src/services/cursor_monitor/mod.rs](../crates/desktop/src/services/cursor_monitor/mod.rs) — `cursor_monitor_logical_center() -> Option<(f32, f32)>` returns `None` on every platform today, with the magnify handler already wiring through `Position::Specific` when the lookup yields a value and falling back to `Position::Centered` otherwise. A drop-in Windows reference implementation (`GetCursorPos` + `MonitorFromPoint` + `GetDpiForMonitor` via `windows-sys`) lives commented at the bottom of that file alongside the dependency stanza needed to enable it. macOS would need `NSEvent::mouseLocation` + `NSScreen::screens()`; Linux X11 needs `XQueryPointer` + Xinerama, Wayland needs the input-method protocol or a DE-specific shim. Without this, side-monitor users always get the launcher on the primary screen.
- **Animate open / close.** Drop in a fade + small Y-translate via the self-driving `RedrawRequested` pattern (mirror `components::toast`). Discrete window-resize on filter change is fine for V1; a 100–150 ms eased height transition would feel snappier still — drive it with `lilt::Animated<f32>` and call `window::resize` per frame while in motion.
- **More copy shortcuts.** TOTP (`Ctrl+T`), URI (`Ctrl+U`), notes (`Ctrl+Shift+N`) — `components::totp::generate_totp` already exists. Footer hint bar gets a "More" expander once there are more than three.
- **Action menu per result.** Tab cycles through copy options, or `Cmd+1/2/3`. Lets users pick non-default fields without leaving the keyboard.
- **Recent / smart ranking.** Sort by last-used; pin manually-favourited items to the top. Requires per-user usage tracking (a small side-table in `data/`).
- **Multi-user search.** Currently single-user (active user's vault). Adding a user badge per row + cross-user merge unlocks "search any unlocked account" UX.
- **Autotype handoff.** When the in-house autotype framework lands ([Tier 1 → Wire up the stubbed settings → Enable autotype]), Enter on the highlighted result triggers it. V1 leaves Enter as a no-op for exactly this reason.
- **Decrypt-failure feedback.** Today an `Err` from `full_cipher` only logs at `warn` and the launcher silently dismisses ([handlers/magnify.rs](../crates/desktop/src/app/handlers/magnify.rs) `PasswordDecryptCompleted`). The main-window toast path doesn't help since the launcher hides itself before completion. If a tray-balloon / OS-notification path gets added later, hook this into it.
- **Toast confirmation on copy.** Designs don't show one and the launcher dismissing is implicit feedback, but a small "Copied" toast on the main window (when visible) would match the rest of the app's copy UX.

#### Code shape

- **Encapsulate `MagnifyView` state.** All fields are `pub(crate)` and the App-level handler reaches in directly ([handlers/magnify.rs](../crates/desktop/src/app/handlers/magnify.rs) writes `magnify.query`, `magnify.selected`, `magnify.scroll_offset_y` etc.). Future refactors of the struct's shape get no compile-time guidance. Move every field to private and expose small mutation methods on `MagnifyView` (`set_query`, `set_selected`, `update_scroll_offset`, `clamp_selection`). Verbose because there are ~10 fields, all read+write — defer until the next refactor that touches the state struct anyway.
- **Light-theme selection contrast.** White text on `#53A3FA` (the `magnify_selected` token) measures ~2.9:1 on the light theme's pale surface — below WCAG AA's 4.5:1 for normal text. Two viable fixes: (a) use `theme.colors.text_primary` for selected-row text instead of hardcoded `Color::WHITE` (adapts per theme, keeps the Figma-fidelity blue), or (b) make `magnify_selected` per-theme — keep `#53A3FA` in dark, swap a darker blue (≈ `#1A73E8`) in light. Option (a) drifts less from the design intent. Dark theme already passes.

#### Documentation

- **Document `AppTheme::with_transparent_background()`.** The per-window theme-variant pattern that lets Magnify return `background: TRANSPARENT` from `iced::theme::Base::base()` is the first of its kind in the codebase. Add a short paragraph under "Theme" in [docs/decisions.md](./decisions.md) explaining when to add further per-window variants — without it a future author writing a second translucent window will reinvent the dispatch-layer trick.
- **Document the broadcast-channel pattern as a shared shape.** `services/{menu,tray,global_hotkey}` all use the same recipe: `static OnceLock<broadcast::Receiver<T>>`, `install_event_handler()` once at startup, `Subscription::run(fn_pointer)` consuming `event_stream()`. Promote to a documented section in [docs/architecture.md](./architecture.md) (or a `docs/skills/` entry) covering the invariants — store `Receiver` and call `.resubscribe()`, fn-pointer subscription identity, handle `RecvError::Lagged`. Otherwise the next external-event source author will pattern-match from whichever service they happen to read first.

### SVG logo antialiasing

Iced's `resvg` rasterizer doesn't match browser quality. Consider a pre-rasterized PNG with 2× / 3× variants, or wait for resvg improvements upstream.

---

## Tier 3 — Research & Architecture

Larger investigations or design decisions that need a written plan before implementation. Each could become its own "plan mode" session.

### Integrate `desktop_native` from the old clients

The `clients/` git submodule already contains `apps/desktop/desktop_native/` — a workspace of pure-Rust Cargo crates that the Electron app calls via NAPI for OS-native features. Two of those modules are directly useful to us:

- **`desktop_core::biometric_v2`** ([clients/apps/desktop/desktop_native/core/src/biometric_v2/](../clients/apps/desktop/desktop_native/core/src/biometric_v2)) — per-platform biometric unlock (Windows Hello, Touch ID, polkit). Modules: `windows.rs`, `windows_focus.rs`, `linux.rs`, `unimplemented.rs` (macOS TBD). Would back a real implementation of the "Unlock with Windows Hello" button currently stubbed to a "not yet supported" toast.
- **`desktop_core::ssh_agent`** ([clients/apps/desktop/desktop_native/core/src/ssh_agent/](../clients/apps/desktop/desktop_native/core/src/ssh_agent)) — an SSH agent that serves keys from the unlocked vault via `russh`. Modules: `unix.rs`, `windows.rs` (named-pipe listener), `request_parser.rs`, `peerinfo`. Depends on `bitwarden-russh` (same pin as the rest of the SDK).

There's also a standalone `ssh_agent` binary crate at `clients/apps/desktop/desktop_native/ssh_agent/` that wraps the core module with an IPC server.

**Investigation questions:**

1. **Is `desktop_core` consumable as a direct Cargo dependency** from our workspace, or is it tangled up with the NAPI layer? Check `lib.rs` — it has module guards but `#[global_allocator] ZeroAlloc` is set at crate root, which may conflict with anything *our* workspace wires up.
2. **License compatibility.** `clients/` is Bitwarden's own repo — check whether `desktop_core` is under the GPLv3 license that applies to `apps/desktop/`, or if it's split out under Bitwarden SDK's Apache/GPL dual license. If `desktop_core` is GPL-only, linking against it would make our binary GPL too.
3. **Integration path options:** (a) direct path dep on the submodule, (b) git-pinned revision, (c) fork and vendor, (d) extract modules into our workspace.
4. **Biometric v2 API surface.** What does Windows Hello require (hwnd? UWP runtime?)? Does it block the UI thread? Is the API `async`?
5. **SSH agent lifecycle.** In-process as a tokio task or out-of-process like the Electron app? What happens on lock/unlock? How does it get keys from the SDK?
6. **Platform coverage gaps.** macOS Touch ID is currently `unimplemented.rs`.

**Deliverable:** a plan document with a recommendation for (integration path) × (biometrics in scope?) × (ssh_agent in scope?).

### Toast API review

Before we grow many more call sites (unlock failure, copy-to-clipboard, sync errors, etc.), validate the API:

- **`Toast::info/success/warning/error(body, title)` surface** — the `title: Option<&str>` slot is a pain point; callers pass `None` in the default case. Alternatives: builder (`Toast::warning("body").with_title("Title")`), default+titled overloads, or drop the title entirely.
- **`Toast` struct schema** — today `{ title, body, status }`. Future callers may want an action button ("Undo"), icon override, sticky flag (no auto-dismiss), custom timeout. Plan the schema before ten call sites exist.
- **Document final design** in `decisions.md`.

**Note:** "toast emission from anywhere" is already solved by compositional MVU — sub-views emit `*Event::ToastRequested(Toast)` events that the App handler routes to `push_toast()`.

### `refresh_cache` invalidation split

`App::post_update()` calls `refresh_cache()` on every message. `refresh_cache` rebuilds `Vec<AccountEntry>` (allocating 3 strings per user), recomputes `unlock_alternatives`, and calls `sync_native_enabled` which iterates every menu item and crosses an FFI boundary on Windows. At 16 ms muda polling this runs ~60×/sec continuously.

**Better shape**: drive invalidation from specific handlers — `ClientManagerLoaded`, `SignOutRequested`, `LockAllVaults`, account-switch, auth-page change. Keep `post_update` as a no-op in the default path. Split `refresh_cache` into per-concern refreshers.

**Prereq for useful measurement**: benchmark both before and after with the loadtest account active and native menus attached.

### Precompute lowercase search keys

`VaultView::filter_items` re-lowercases `name`, `subtitle`, and URI per item per keystroke. On the 20 k loadtest account that's ~60 k allocations per keystroke. Wrap `CipherListView` in a `CipherRow { inner: Arc<CipherListView>, name_lc: String, subtitle_lc: String, uri_lc: Option<String> }` populated once on `ListLoaded`. Filter against pre-lowered strings. Pairs well with the `Arc<[CipherListView]>` micro-optimization in Tier 4.

### `UpdateCtx` filter leak (revisit when a 3rd filter lands)

[`app/ctx.rs`](../crates/desktop/src/app/ctx.rs) currently carries `active_vault_filter` + `active_send_filter` as separate fields on `UpdateCtx`. Every view's `update()` receives both, even views that don't consume either. The Generator landed without a sidebar filter (history mode is internal to the modal), so we're still at N=2; whichever next view introduces a sidebar-driven filter will tip this into N=3 and the right move becomes clear:

- **Option A:** collapse to a single `sidebar: &SidebarState` field — views that care destructure what they need.
- **Option B:** push-based: App dispatches a `FilterChanged { filter }` message into the affected view on sidebar clicks rather than threading the current filter through every update cycle.

Defer until N=3 so the right split is obvious instead of guessed.

### In-form validation surface (red borders + per-field hints)

Today both `CipherForm` and `SendForm` use a single `is_valid()` method + a `toast-required-fields` toast on save. That gets thin once required-field rules grow past one field. Upgrade shape:

- Replace `is_valid() -> bool` with `validate() -> HashMap<FieldId, &'static str>` (or `Vec<(FieldId, &'static str)>`) returning per-field error messages.
- Add a `show_validation: bool` flag on each form, flipped to `true` the first time the user clicks Save with an invalid form. Keeps first-view UX clean — fields only turn red *after* the user has expressed intent to save.
- Add an `inputs::validated_text_field(...)` / style helper in [`components/inputs.rs`](../crates/desktop/src/components/inputs.rs) that takes `Option<&str>` error. When `Some`, it paints a red border via the `text_input::Style` closure (iced's style closure receives `&Theme, Status` so the color swap is a one-liner) and renders the error string below the field in the secondary-muted red.
- Keep the toast as a global "can't save yet" nudge, but demote it to the title — the specifics live inline under each field.

Worth doing once a second required field lands on either form; overkill for one `name` field.

### Prune dead i18n keys

`assets/i18n/{lang}/bitwarden_desktop_next.ftl` files grow additively. `i18n-embed-fl` validates Rust → `.ftl` references at compile time, but unused keys in the `.ftl` file itself are silent. Add a small lint (grep-based check, or a `cargo xtask i18n-unused` that parses `.ftl` IDs and greps the Rust tree) before the file crosses ~200 keys. Today it's ~130.

### Vault + Send event handlers will generalize once a 3rd list view exists

[`vault/handler.rs`](../crates/desktop/src/views/vault/handler.rs) and [`send/handler.rs`](../crates/desktop/src/views/send/handler.rs) both dispatch the same five event arms — `AccountSwitcher` / `ToastRequested` / `ItemSaved` / `ItemDeleted` / `ClipboardCopyRequested` — differing only in toast strings and which list-reload task to call. Pairs naturally with the "Shared list-view primitive" entry below: once that lands, the handlers deduplicate for free via a shared `ListEvent<V>` or similar. Don't fix in isolation — that just introduces an abstraction the list-view primitive will replace.

### Shared list-view primitive (revisit when a 3rd list view lands)

`VaultView` and `SendView` share near-identical structure: a `Selection { item, id, form, confirm_delete }` / `ItemCache { all, cached }` pair keyed by `UserId`, plus a `recompute_filtered` + `filter_items` + search-query pattern and a `CollapsiblePane` holding the list + detail/form split. Both views also reimplement the same `apply_filter` / `reset` / `remove_user_items` / `focus_search_task` methods.

Two implementations is a coincidence; three is a pattern. Before `Generator` (or any future list-style screen) lands, evaluate extracting:

- A `ListView<T, Form>` generic state struct holding `Selection<T::Id, Form>` + `HashMap<UserId, ItemCache<T>>` + `CollapsiblePane` + scroll + search query, with `recompute_filtered` generic over a filter predicate.
- A `ListViewController` trait with `load_list_task`, `full_item`, `save_item`, `delete_item` SDK shims, so the state struct can drive the async flow without knowing about ciphers or sends specifically.

**Defer until we have a concrete third consumer** — premature abstraction here would lock the shape against a use case we haven't seen yet. Revisit alongside the Tier 1 "Implement the login command" / Generator work.

### Testing infrastructure

- **Unit tests for pure logic** — `VaultView::filter_items`, `Shortcut::matches`, `EnabledWhen::check`, sub-view `update()` state machines (message + injected deps → task + event). Standard `#[test]`, no framework needed.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

---

## Tier 4 — Deferred / Waiting Upstream

### SSH agent — waiting on upstream V2

`Settings::ssh_agent` (master toggle) and the per-user `SshPromptBehavior` (Always / Never / RememberUntilLock) are already wired through [crates/desktop/src/views/settings/tabs/integrations.rs](../crates/desktop/src/views/settings/tabs/integrations.rs) and persisted, but the agent itself isn't started — toggling currently no-ops on the runtime side.

Upstream has two implementations and neither is a clean target right now:

- **V1** ([clients/apps/desktop/desktop_native/core/src/ssh_agent/](../clients/apps/desktop/desktop_native/core/src/ssh_agent)) is functional (named pipe on Windows, Unix socket elsewhere; `bitwarden-russh` for the agent protocol; `mpsc` request → `broadcast` response channels for UI approval — maps cleanly onto our `Subscription::run` pattern). But the module header flags it deprecated, accepting only security patches until V2 lands.
- **V2** ([clients/apps/desktop/desktop_native/ssh_agent/](../clients/apps/desktop/desktop_native/ssh_agent)) has the cleaner shape we'd want to build against (`ApprovalRequester` async trait, generic `KeyStore`, separate crate), but `BitwardenSSHAgent::start_server()` is a no-op stub (PM-30756) and `InMemoryEncryptedKeyStore::sign_data()` is `todo!()` (PM-30755). Not usable yet.

**Decision:** park until V2 ships. Building on V1 now would mean rewriting most of it against V2 within months. Revisit when the upstream `todo!()` calls are gone and V2 is shipped in the Electron client.

When unblocked: implement `ApprovalRequester` so approval prompts pump onto a tokio broadcast channel consumed by an `iced::Subscription::run` stream (mirrors [services/instance_lock](../crates/desktop/src/services/instance_lock/mod.rs)); the prompt opens a small confirmation window (About-window pattern in [app/handlers/platform.rs:225](../crates/desktop/src/app/handlers/platform.rs#L225)) when `SshPromptBehavior` requires it; `Settings::ssh_agent` toggles drive the listener task lifecycle; the keystore is fed from the active user's `CipherType::SshKey` ciphers via `ClientManager`. The SDK's `bitwarden-ssh` crate handles key generation/import/export only — it is **not** the agent protocol layer (that's `bitwarden-russh`).

### Hot reloading

Iced [PR #3000](https://github.com/iced-rs/iced/pull/3000) is merged into master (June 2025). Uses `hot` feature flag + `subsecond` / `cargo-hot`. Available on 0.15. Worth trying on a feature branch — no core architecture changes needed.

### Scrollbar minimum thumb height (upstream PR candidate)

With 20 k+ items in a virtualized list, iced's scroll thumb shrinks to its hardcoded 2 px minimum and becomes visually imperceptible / ungrabbable. The constant is in `iced_widget-0.15/src/scrollable.rs`:

```rust
let scroller_height = (scrollbar_bounds.height * ratio).max(2.0);
```

`Scrollbar` exposes `width`, `margin`, `scroller_width`, `alignment`, `spacing` — but not `min_scroller_height` / `min_scroller_length`.

**Upstream PR shape:**
1. Add `min_scroller_length: f32` field (default `2.0` to preserve current behavior) to `Scrollbar` struct.
2. Add a `min_scroller_length(impl Into<Pixels>) -> Self` builder method.
3. Replace `.max(2.0)` with `.max(self.min_scroller_length)` in both the vertical and horizontal branches.

**Workaround until this lands:** either vendor `scrollable.rs` locally (~2 400 LOC) or overlay a custom thumb next to a `Scrollbar::hidden()` scrollable (~150–250 LOC new widget).

**Decision:** wait on upstream.

### `Arc<[CipherListView]>` micro-optimization

Today `VaultView.items.all` and `.cached` hold `Vec<Arc<CipherListView>>`, paying a refcount bump per item on every clone/filter pass. The whole vec could share one allocation: `Arc<[CipherListView]>`, with filter/search operating on indices into the shared slice. The `VaultMessage::ListLoaded` variant would simplify to `Result<Arc<[CipherListView]>, String>`.

**Blocker:** needs before/after measurement on the loadtest account. Pair with the Tier 3 lowercase search key caching for a bigger perf win.

### LoginView per-AuthPage field grouping

`AuthPage` enum carries all per-page state as inline fields; 15 match arms in `LoginView::update` start with `if let AuthPage::Unlock { ... } = &mut self.auth_page`. As the login flow grows (Registration, SSO, hint), the fields accumulate. Extract into `UnlockState` / `LoginEmailState` / `LoginPasswordState` sub-structs. Wait until touching `LoginView` for Registration work — doing both at once is cheaper.

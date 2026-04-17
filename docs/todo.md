# TODO

Tasks are organized into four rough tiers by priority. Within a tier, pick what's
most unblocking or most interesting — the tiers are not a strict ordering.

## Contents

- [Recently Completed](#recently-completed)
- [Tier 1 — Do Next](#tier-1--do-next)
- [Tier 2 — User-Visible Features](#tier-2--user-visible-features)
- [Tier 3 — Research & Architecture](#tier-3--research--architecture)
- [Tier 4 — Deferred / Waiting Upstream](#tier-4--deferred--waiting-upstream)

---

## Recently Completed

- **Iced 0.15 upgrade + CPU renderer default.** `iced = "0.15"` (git-pinned), default backend `tiny-skia` — saves ~500 ms startup + ~9 MB binary vs wgpu. `--gpu` runtime flag in `main.rs` is gated by `cfg!(feature = "gpu")`; enable wgpu by uncommenting `"wgpu"` in the iced features list in `crates/desktop/Cargo.toml`.
- **Lazy data load + Loading screen.** `App::new` returns fast with `ClientManager::empty()` and `Screen::Loading`; 24 MB JSON parse runs on tokio's blocking pool via `spawn_blocking`. When `SystemMessage::ClientManagerLoaded` arrives, the handler swaps the `Arc` and transitions to `Screen::Login`. New `components/spinner.rs` is a self-animating 8-dot ring widget (same `RedrawRequested` + `request_redraw_at` pattern as the toast overlay, no app-level subscription).
- **Unlock loading indicator.** `LoginView.unlock_in_progress: bool` drives an inline spinner inside the primary button during the unlock task. Password input becomes read-only, alternate buttons + Log out stay visible but inert. Re-entry guards in `LoginMessage::Unlock` prevent Enter-spam. Flag cleared on `UnlockCompleted` (Ok + Err), `SwitchUnlockMethod`, `reset_to_email_entry`, `show_unlock_for`.
- **Skill-driven code review pass.** Ran code-architect, code-explorer, code-reviewer, and simplify over the whole project; written reports under `docs/skills/`. High-confidence bug / quality / efficiency findings landed as concrete fixes: sign-out screen transition, `MemoryRepo` mutex-poison-to-`RepositoryError::Internal` mapping, scrollbar thumb theme token, `AppTheme.name` → `&'static str`, toast string clone removal, SDK error sanitization in unlock-failure toast, `SEARCH_ID` widget-id constant, `field_readonly` helper unification, `load_vault_list_task` factory moved onto `VaultView`, dead `has_authenticated_accounts` menu-state bool collapsed, Linux dead-panic removed, spurious `SearchFocusRequested` spam on every keystroke removed, `BackToEmail` now preserves the typed email, `DetailLoaded` decrypt errors now toast instead of silently logging.
- **Virtual List / Lazy Scrolling.** New generic `components::virtual_list` module implements DIY viewport-windowed scrolling over uniform-height items: builds widgets only for rows inside the visible window (+ `MIN_OVERSCAN = 5` rows of overscan), with `Space::with_height` fillers above/below to preserve the scroll thumb ratio. Drops `Column::layout()` cost from O(N) to O(window ≈ 20). `item_list::view` is the first consumer — each row is wrapped in a `Length::Fixed(49)` container.
- **Stable SDK `UserId` per Client.** `UserEntry::sdk_user_id` is parsed once from the mock vault's stable UUID string (`11111111-1111-4111-a111-...` and friends) and reused on every unlock. Fixed the latent bug where `UserId::new_v4()` was generated fresh on every `unlock()` call.
- **Observability: `tracing` + `tracing_subscriber`.** Workspace deps added; subscriber installed in `main.rs::init_tracing()` driven by `RUST_LOG` (default `bitwarden_desktop_next=debug,warn`). All `eprintln!` call sites converted to structured `tracing::info!` / `warn!` / `error!` / `debug!` with named fields. SDK `#[tracing::instrument]` spans visible for free under `RUST_LOG=...,bitwarden_core=debug`.
- **Load-test account (~20 k ciphers).** Third user `loadtest@example.com` / password `loadtest` in `tools/fake-data`. Deterministic xorshift64 PRNG drives name/username picks. JSON output switched to compact form — `assets/mock-vault.json` is now ~24 MB.
- **View Encapsulation refactor (compositional MVU).** Sub-views return `(Task<SubMessage>, Option<SubEvent>)`; async lifecycle lives with the owning view; `app.rs::update` is a pure router. See [decisions.md](./decisions.md) → "Compositional MVU".
- **`App` struct regrouped** into `ViewCache` / `ThemeState` / `WindowInfo` sub-structs.
- **`VaultView` regrouped** into `SidebarState` / `Selection` / `ItemCache` sub-structs. `sidebar::view` / `expanded_panel` signatures dropped from 6 and 5 params to 2 each.
- **`view()` functions are now methods** on their view structs.
- **`Border` / `container::Style` builder pattern adoption.** `Border::default().color(c).width(w).rounded(r)` and `container::Style::default().background(c).border(b)` replaced `{ ..Default::default() }` struct-literal boilerplate across 14 files. Style construction is shorter and less error-prone.

---

## Tier 1 — Do Next

These are the highest-value tasks with no architectural prerequisites. Doing them soon unblocks everything else or closes UX gaps users would notice immediately.

### Wire copy buttons in detail pane to clipboard

Use `arboard` crate or iced clipboard API. The detail pane buttons exist but are stubs today — this is the most-noticed missing feature when manually testing.

### Mask SSH private keys and card CVVs

Currently `views/vault/widgets/detail_pane.rs` renders `key.private_key` and `card.code` as plaintext. Match the official app: default to masked (bullets), show a reveal toggle (eye icon) per field. Reuse the password-field reveal pattern from `input_field.rs` as a starting point, or factor out a `reveal_field(label, value, revealed, on_toggle)` helper once two fields want it.

### Sanitize remaining SDK error echoes into toasts

`LoginMessage::UnlockCompleted` now shows "Check your master password and try again" instead of the raw SDK error. Apply the same treatment to any future toast paths that surface SDK errors. Rule: raw `e.to_string()` goes to `tracing::warn!`/`error!`, the user sees a short sanitized string.

### `UserId` newtype

`state::UserId = String` today. The SDK's `bitwarden_core::UserId` is a distinct newtype; our alias accepts any string. Wrap as `pub struct UserId(String)` with `Clone`, `Debug`, `PartialEq`, `Eq`, `Hash`, `Display`, `Deref<Target=str>`, `AsRef<str>`, `From<String>`. rustc guides the migration; closes the "any string accepted" gap. Once done, consider formalizing the boundary so the newtype carries a parsed `uuid::Uuid` internally and `ClientManager` keys off that.

---

## Tier 2 — User-Visible Features

Features that complete the happy paths users expect. No architectural work required — each is a contained feature addition following the "how to add a new view" recipe.

### Auth flow completion

- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields. New sub-view under `views/register/` plus a new `Register` screen variant or inlined into `LoginView`.
- **Master password hint request** — "Get master password hint" link on login password screen sends a hint request to the server.
- **Self-hosted server URL modal** — server selector's "Self-hosted" option should open a modal to input custom server URL. (Depends on the modal framework work in Tier 3.)
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

### Detail pane completion

- **Wire edit/delete buttons** — currently stubs. Edit opens an editor (modal or new screen); delete confirms then calls the SDK.
- **TOTP circular timer** — currently placeholder text.

### Account switcher polish

- **Avatar color auto-generation** — generate the avatar background color from a username/email hash (matches the official app).
- **Account switcher dropdown visual update** — match 2025 Figma (Lock/Logout buttons, Options section).
- **Shared behavioral helper** — `AccountSwitcherMessage` handling is ~15 lines duplicated in `LoginView::update` and `VaultView::update`. Extract a `handle_switcher_msg(msg, &mut open) -> SwitcherOutcome` free function in `components/account_switcher.rs` that returns a small outcome enum each view maps to its own event type. Only worth doing when a third caller appears or when the logic diverges.

### Tray icon

Use the `tray-icon` crate (sister to `muda`, same raw window handle approach). Show the Bitwarden shield icon with a right-click context menu (Lock / Quit). Handle tray-icon click to show/hide the main window.

**Note:** iced [PR #3021](https://github.com/iced-rs/iced/pull/3021) adds native tray support but is still open (targeting 1.0), so use the `tray-icon` crate directly for now.

### Sidebar polish

- **Indented tree hierarchy** — Vault > All vaults > My vault, with visual indent.
- **Expand/collapse animation** — needs `Subscription` tick + interpolated width, or the self-animating widget pattern extended to container layout.

### SVG logo antialiasing

Iced's `resvg` rasterizer doesn't match browser quality. Consider a pre-rasterized PNG with 2× / 3× variants, or wait for resvg improvements upstream.

---

## Tier 3 — Research & Architecture

Larger investigations or design decisions that need a written plan before implementation. Each could become its own "plan mode" session.

### Password / PIN zeroize

`AuthPage::Unlock { password_input: String, pin_input: String, ... }` and `AuthPage::LoginPassword { password_input: String, ... }` hold sensitive credentials as plain `String`s that aren't zeroed on drop. `std::mem::take` moves the value but leaves the allocator free to reuse the backing bytes without clearing. Minimum viable fix: depend on `zeroize` (already a transitive dep via `bitwarden-crypto`), change the field type to `zeroize::Zeroizing<String>`. Also consider: clear inputs proactively on screen switch, on `LockAllVaults`, on app suspend. The iced `text_input` widget still keeps its own `Value` buffer that isn't zeroable without forking — see Tier 4 "Secure Text Input" for that piece.

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

### Modal framework

For item editor, password generator, settings forms, confirmations, and the self-hosted server URL modal in Tier 2. Iced's official [modal example](https://github.com/iced-rs/iced/blob/master/examples/modal/src/main.rs) shows a clean pattern:

- `modal(base, content, on_blur)` helper using `stack!` to layer: base → `opaque()` dark overlay → centered modal content. `mouse_area` on the overlay detects click-outside and fires `on_blur`.
- Escape key handled separately in `update()` via `keyboard::Event::KeyPressed { key: Named(Escape) }`.
- Conditional rendering: `if self.show_modal { modal(content, dialog, Msg::Hide) } else { content.into() }`.

**Pros over popup windows:** no window lifecycle complexity, works on all platforms identically, simpler state.
**Cons:** can't drag the modal out, limited to one overlay level (iced constraint — same as our DropDown fork).

**When to use which:** modal for item editor / password generator / settings / confirmations. Popup window for side-by-side vault item comparison.

**Deliverable:** a `components::modal` helper + ADR in `decisions.md`.

### Multi-Window Support (already daemon-ready)

Architecture is already on `iced::daemon`; `WindowMessage` variants carry `window::Id` already. Migration to real multi-window use would:

- Open additional windows via `iced::window::open` in whichever handler fires the open request (like the About window does today).
- `view(&self, window::Id) -> Element` already branches on the id.
- Any per-window state (per-window scroll, per-window selection) needs to live in the `windows: HashMap<window::Id, WindowInfo>` entry.

**Blocker:** no concrete use case yet. Modal framework above covers most needs and is cheaper. Flagged in [Zulip](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/.E2.9C.94.20Support.20for.20per-window.20views/with/577450159).

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

### Event-driven muda bridge

`PollNativeMenu` subscription fires every 16 ms when `native_menu.is_some()` — a continuous update churn even when no menu event happened. Replace with a channel-backed `Subscription::run` wired to `muda::MenuEvent::receiver()` so only real menu events wake `update`. Architectural change; low ROI until perf measurement actually flags it.

### Precompute lowercase search keys

`VaultView::filter_items` re-lowercases `name`, `subtitle`, and URI per item per keystroke. On the 20 k loadtest account that's ~60 k allocations per keystroke. Wrap `CipherListView` in a `CipherRow { inner: Arc<CipherListView>, name_lc: String, subtitle_lc: String, uri_lc: Option<String> }` populated once on `ListLoaded`. Filter against pre-lowered strings. Pairs well with the `Arc<[CipherListView]>` micro-optimization in Tier 4.

### Shared `mock-vault.json` schema crate

`MockVaultFile` / `MockUser` / `UnlockMethodsCfg` are defined independently in `crates/desktop/src/sdk.rs` and `tools/fake-data/src/main.rs`, with comments instructing the reader to keep both in sync. Extract into `tools/mock-schema` with `Serialize + Deserialize`; both crates add it as a path dep. Closes the silent-drift risk on the 24 MB committed artifact.

### Testing infrastructure

- **Unit tests for pure logic** — `VaultView::filter_items`, `Shortcut::matches`, `EnabledWhen::check`, sub-view `update()` state machines (message + injected deps → task + event). Standard `#[test]`, no framework needed.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

---

## Tier 4 — Deferred / Waiting Upstream

### Secure Text Input

Iced's `text_input` uses a standard `String` internally that isn't scrubbed on drop. Master passwords may linger in heap memory after reallocation.

**Practical impact is debatable** — an attacker with enough access to probe process memory likely has easier vectors (keyloggers, `/proc/mem`, input ring buffers). Memory scrubbing is defense-in-depth, not a primary control. See [Zulip discussion](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Secure.20text_input/with/221416543).

**Our SDK already helps** — zeroizing allocator scrubs secrets passed *through* SDK types. The gap is specifically iced's `text_input::Value` before the password reaches the SDK. The Tier 3 "Password / PIN zeroize" item is the fix for the `AuthPage` side; `text_input` itself still needs upstream work or a fork.

**Options:** (1) fork `text_input` / `value.rs` to use `zeroize::Zeroizing<String>` — maintenance burden on iced upgrades. (2) Minimize exposure window — copy out into a zeroizing type immediately on submit (already done via `std::mem::take`), clear input field on page transitions. (3) Accept the gap.

**Decision:** low priority. Approach (2) is essentially free — done. Approach (1) only if compliance requires it.

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

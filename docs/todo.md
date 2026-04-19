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

These are the highest-value tasks with no architectural prerequisites. Doing them soon unblocks everything else or closes UX gaps users would notice immediately.

### Sanitize remaining SDK error echoes into toasts

Rule for future toast paths that surface SDK errors: raw `e.to_string()` goes to `tracing::warn!`/`error!`, the user sees a short sanitized string. `UnlockCompleted` and `LoginCompleted` both follow this pattern today.

### Settings view with a language dropdown

Call `i18n_embed::select(...)` on change (re-renders automatically on iced's next frame). Design question: where the settings screen lives in the nav (titlebar menu item, gear icon in sidebar, etc.) and whether it's a window/modal/view.

Card-brand pick-list strings in `cipher_form.rs` are intentionally left untranslated — the canonical brand names ("Visa", "Mastercard", …) are not localized.

---

## Tier 2 — User-Visible Features

Features that complete the happy paths users expect. No architectural work required — each is a contained feature addition following the "how to add a new view" recipe.

### Auth flow completion

- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields. New sub-view under `views/register/` plus a new `Register` screen variant or inlined into `LoginView`.
- **Master password hint request** — "Get master password hint" link on login password screen sends a hint request to the server.
- **Self-hosted server URL modal** — server selector's "Self-hosted" option should open a modal to input custom server URL. (Depends on the modal framework work in Tier 3.)
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

### Account switcher polish

- **Avatar color auto-generation** — generate the avatar background color from a username/email hash (matches the official app).
- **Account switcher dropdown visual update** — match 2025 Figma (Lock/Logout buttons, Options section).
- **Shared behavioral helper** — `AccountSwitcherMessage` handling is ~15 lines duplicated in `LoginView::update` and `VaultView::update`. Extract a `handle_switcher_msg(msg, &mut open) -> SwitcherOutcome` free function in `components/account_switcher.rs` that returns a small outcome enum each view maps to its own event type. Only worth doing when a third caller appears or when the logic diverges.

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
- **Expand/collapse animation** — needs `Subscription` tick + interpolated width, or the self-animating widget pattern extended to container layout.

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

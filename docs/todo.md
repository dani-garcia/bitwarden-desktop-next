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

- **Virtual List / Lazy Scrolling.** New generic `components::virtual_list` module implements DIY viewport-windowed scrolling over uniform-height items: builds widgets only for rows inside the visible window (+ 5-row overscan), with `Space::with_height` fillers above/below to preserve the scroll thumb ratio. Drops `Column::layout()` cost from O(N) to O(window ≈ 20). `item_list::view` is the first consumer — each row is now wrapped in a `Length::Fixed(49)` container for a uniform height, and a `ListScroll` `ScrollState` field on `VaultView` is updated from `ItemListMessage::Scrolled`. Reused on any future >1k list (password history, device sessions, etc.).
- **Stable SDK `UserId` per Client.** `UserEntry::sdk_user_id` is parsed once from the mock vault's stable UUID string (`11111111-1111-4111-a111-...` and friends) and reused on every unlock. Fixed the latent bug where `UserId::new_v4()` was generated fresh on every `unlock()` call, which confused the SDK's Client ↔ user binding. `uuid` crate hoisted to workspace deps.
- **Observability: `tracing` + `tracing_subscriber`.** Workspace deps added; subscriber installed in `main.rs::init_tracing()` driven by `RUST_LOG` (default `bitwarden_desktop_next=debug,warn`). All `eprintln!` call sites in `app.rs`, `views/login/mod.rs`, `views/vault/mod.rs` converted to structured `tracing::info!` / `warn!` / `error!` / `debug!` with named fields. `ClientManager::load()` logs per-user counts. The SDK's own `#[tracing::instrument]` spans are now visible for free once the subscriber is installed.
- **Load-test account (~20k ciphers).** Third user `loadtest@example.com` / password `loadtest` in `tools/fake-data/src/main.rs`. Deterministic xorshift64 PRNG (seed `0xC0FFEE_DEADBEEF`) drives name/username picks from small word banks. Type mix: 80% logins / 10% notes / 5% cards / 3% identities / 2% SSH keys. JSON output switched to compact form — `assets/mock-vault.json` is now ~24MB (still embeddable via `include_bytes!`).
- **View Encapsulation refactor.** Sub-views now return `(Task<SubMessage>, Option<SubEvent>)`, async lifecycle lives with the owning view, `app.rs::update` is a ~60-line router. See [decisions.md](./decisions.md) → "View Architecture: Compositional MVU" and [architecture.md](./architecture.md) → "How to Add a New View".
- **`App` struct regrouped** into `ViewCache` / `ThemeState` / `WindowState` sub-structs. Deleted redundant `menu_attached` flag.
- **`VaultView` regrouped** into `SidebarState` / `Selection` / `ItemCache` sub-structs. Collapsed `sidebar::view` / `expanded_panel` signatures from 6 and 5 params down to 2 each. Fixed a latent bug where `SidebarMessage::FilterSelected` didn't refresh `cached_items`.
- **`view()` functions are now methods** on their view structs (`LoginView::view`, `VaultView::view`, `TitleBarState::view`). `vault::view` dropped from 17 params to 4.
- **`UserEntry.email` duplication deleted** — `unlock()` now reads `entry.metadata.email` from the single source of truth.

---

## Tier 1 — Do Next

These are the highest-value tasks with no architectural prerequisites. Doing them soon unblocks everything else or closes UX gaps users would notice immediately.

### Wire copy buttons in detail pane to clipboard

Use `arboard` crate or iced clipboard API. The detail pane buttons exist but are stubs today — this is the most-noticed missing feature when manually testing.

### Unlock loading indicator

From the moment unlock fires until either `VaultMessage::ListLoaded` arrives (`Ok`) or `LoginMessage::UnlockCompleted` arrives (`Err`), the UI sits frozen on the unlock screen with no feedback. Replace the "Unlock" button with a spinner (probably `iced::widget::progress_bar` in indeterminate mode or a custom rotating icon), disable the input, hide the alternate-unlock-method links.

**Implementation notes:**
- Add a small `unlock_in_progress: bool` on `LoginView`.
- Set it `true` when `LoginMessage::Unlock` fires `Task::perform`.
- Clear it in the `LoginMessage::UnlockCompleted` handler on both `Ok` and `Err` paths.
- View reads it to switch between "Unlock" button and spinner.

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

### Tray icon

Use the `tray-icon` crate (sister to `muda`, same raw window handle approach). Show the Bitwarden shield icon with a right-click context menu (Lock / Quit). Handle tray-icon click to show/hide the main window.

**Note:** iced [PR #3021](https://github.com/iced-rs/iced/pull/3021) adds native tray support but is still open (targeting 1.0), so use the `tray-icon` crate directly for now.

### Sidebar polish

- **Indented tree hierarchy** — Vault > All vaults > My vault, with visual indent.
- **Expand/collapse animation** — iced 0.14 has no built-in layout transitions; needs `Subscription` tick + interpolated width.

### SVG logo antialiasing

Iced's `resvg` rasterizer doesn't match browser quality. Consider a pre-rasterized PNG with 2× / 3× variants, or wait for resvg improvements upstream.

---

## Tier 3 — Research & Architecture

Larger investigations or design decisions that need a written plan before implementation. Each could become its own "plan mode" session.

### Integrate `desktop_native` from the old clients (NEW)

The `clients/` git submodule already contains `apps/desktop/desktop_native/` — a workspace of pure-Rust Cargo crates that the Electron app calls via NAPI for OS-native features. Two of those modules are directly useful to us:

- **`desktop_core::biometric_v2`** ([clients/apps/desktop/desktop_native/core/src/biometric_v2/](../clients/apps/desktop/desktop_native/core/src/biometric_v2)) — per-platform biometric unlock (Windows Hello, Touch ID, polkit). Modules: `windows.rs`, `windows_focus.rs`, `linux.rs`, `unimplemented.rs` (macOS TBD). Would back a real implementation of the "Unlock with Windows Hello" button currently stubbed to a "not yet supported" toast.
- **`desktop_core::ssh_agent`** ([clients/apps/desktop/desktop_native/core/src/ssh_agent/](../clients/apps/desktop/desktop_native/core/src/ssh_agent)) — an SSH agent that serves keys from the unlocked vault via `russh`. Modules: `unix.rs`, `windows.rs` (named-pipe listener), `request_parser.rs`, `peerinfo`. Depends on `bitwarden-russh` (same pin as the rest of the SDK).

There's also a standalone `ssh_agent` binary crate at `clients/apps/desktop/desktop_native/ssh_agent/` that wraps the core module with an IPC server.

**Investigation questions:**

1. **Is `desktop_core` consumable as a direct Cargo dependency** from our workspace, or is it tangled up with the NAPI layer? Check `lib.rs` — it has module guards but `#[global_allocator] ZeroAlloc` is set at crate root, which may conflict with anything *our* workspace wires up.
2. **License compatibility.** `clients/` is Bitwarden's own repo — check whether `desktop_core` is under the GPLv3 license that applies to `apps/desktop/`, or if it's split out under Bitwarden SDK's Apache/GPL dual license. Our crate is MIT or whatever we pick; if `desktop_core` is GPL-only, we can't link against it without making our binary GPL too. Verify in `clients/apps/desktop/desktop_native/LICENSE` or the crate-level license field.
3. **Integration path options:**
   - (a) Add `desktop_core = { path = "../../../../clients/apps/desktop/desktop_native/core" }` directly — tightest coupling, tracks upstream `main` automatically.
   - (b) Git-pin a revision of the upstream repo as a git dep — more stable, but requires manual bumps.
   - (c) Fork and vendor the modules we want — fullest control, highest maintenance burden.
   - (d) Extract the two modules into a separate crate in *our* workspace and track upstream by hand — lets us strip dependencies we don't need (we don't care about `autofill_provider`, `chromium_importer`, `process_isolation`, etc.).
4. **Biometric v2 API surface.** What does it look like? What does the Windows implementation require (hwnd? UWP runtime?)? Does it block the UI thread? Is the API `async`?
5. **SSH agent lifecycle.** Does it run in-process inside our app (spawned as a tokio task) or as a separate process (like the current Electron app does — the `ssh_agent` binary is launched and talks over IPC)? What happens when the vault locks — does the agent unregister its socket? How does it get keys from the SDK — does it hold an `Arc<PasswordManagerClient>` or pull from a repository?
6. **Platform coverage gaps.** macOS Touch ID in `biometric_v2` is currently `unimplemented.rs`. We'd need to either fill that gap or gate the Unlock-with-Biometrics button behind `cfg!(target_os)` checks.

**Deliverable:** a plan document with a recommendation for (integration path) × (biometrics in scope?) × (ssh_agent in scope?), plus a list of upstream contributions we'd be making if we go with (a) or (b) — pure-rust desktop client would be the first consumer of `desktop_core` outside the Electron bundle.

### Modal framework

For item editor, password generator, settings forms, confirmations, and the self-hosted server URL modal in Tier 2. Iced's official [modal example](https://github.com/iced-rs/iced/blob/master/examples/modal/src/main.rs) shows a clean pattern:

- `modal(base, content, on_blur)` helper using `stack!` to layer: base → `opaque()` dark overlay → centered modal content. `mouse_area` on the overlay detects click-outside and fires `on_blur`.
- Escape key handled separately in `update()` via `keyboard::Event::KeyPressed { key: Named(Escape) }`.
- Conditional rendering: `if self.show_modal { modal(content, dialog, Msg::Hide) } else { content.into() }`.

**Pros over popup windows:** no daemon migration, no window lifecycle, works on all platforms identically, simpler state.
**Cons:** can't drag the modal out, can't view side-by-side, limited to one overlay level (iced constraint — same as our DropDown fork).

**When to use which:** modal for item editor / password generator / settings / confirmations. Popup window for side-by-side vault item comparison (requires daemon migration — see below).

**Deliverable:** a `components::modal` helper + ADR in `decisions.md`.

### Multi-Window Support

Prerequisite research is done; the architecture is already daemon-ready (the `WindowMessage` variants carry `window::Id`). Migrating requires:

- Swap `iced::application(...)` → `iced::daemon(App::boot, App::update, App::view)` in `main.rs`.
- `boot()` must call `window::open()` and store the returned `window::Id` (we already have `App.window.id: Option<window::Id>`).
- `view(&self, window::Id) -> Element` gains a `window::Id` parameter and branches on it (main window vs. child).
- `subscription()` must listen to `window::close_events()` and call `iced::exit()` when the main window closes — otherwise the app becomes an orphan process.

**Reference:** [iced/examples/multi_window/src/main.rs](https://github.com/iced-rs/iced/blob/master/examples/multi_window/src/main.rs) uses `HashMap<window::Id, Window>` to track per-window state.

**Blocker:** don't do this until we have a concrete use case that needs side-by-side windows. The modal framework above covers most needs and is cheaper. Flagged in [Zulip](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/.E2.9C.94.20Support.20for.20per-window.20views/with/577450159).

### Toast API review

Before we grow many more call sites (unlock failure, copy-to-clipboard, sync errors, etc.), validate the API:

- **`Toast::info/success/warning/error(body, title)` surface** — the `title: Option<&str>` slot is a pain point; callers pass `None` in the default case. Alternatives: builder (`Toast::warning("body").with_title("Title")`), default+titled overloads, or drop the title entirely. Pick whichever reads cleanest.
- **Per-view toast managers vs. single app-level** — `Manager` currently wraps `column![title_bar, page]`. Does that cover everything we'll need, or do we want per-view managers?
- **`Toast` struct schema** — today `{ title, body, status }`. Future callers may want an action button ("Undo"), icon override, sticky flag (no auto-dismiss), custom timeout. Plan the schema before ten call sites exist.
- **Document final design** in `decisions.md`.

**Note:** "toast emission from anywhere" is already solved by the compositional MVU refactor — sub-views emit `*Event::ToastRequested(Toast)` events that the App handler routes to `push_toast()`. Only the API-shape questions above are still open.

### Testing infrastructure

- **Unit tests for pure logic** — `VaultView::filtered_items()`, `Shortcut::matches()`, `EnabledWhen::check()`, sub-view `update()` state machines (message + injected deps → task + event). Standard `#[test]`, no framework needed. The compositional MVU refactor makes these trivial to write — views are constructible without App boot.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test = "0.14"` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

---

## Tier 4 — Deferred / Waiting Upstream

### Secure Text Input

Iced's `text_input` uses a standard `String` internally that isn't scrubbed on drop. Master passwords may linger in heap memory after reallocation.

**Practical impact is debatable** — an attacker with enough access to probe process memory likely has easier vectors (keyloggers, `/proc/mem`, input ring buffers). Memory scrubbing is defense-in-depth, not a primary control. See [Zulip discussion](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Secure.20text_input/with/221416543).

**Our SDK already helps** — zeroizing allocator scrubs secrets passed *through* SDK types. The gap is specifically iced's `text_input::Value` before the password reaches the SDK.

**Options:** (1) fork `text_input` / `value.rs` to use `zeroize::Zeroizing<String>` — maintenance burden on iced upgrades. (2) Minimize exposure window — copy out of the text input into a zeroizing type immediately on submit, clear the input field (already done for UX reasons). (3) Accept the gap.

**Decision:** low priority. Approach (2) is essentially free — verify we're doing it. Approach (1) only if compliance requires it.

### Hot reloading

Iced [PR #3000](https://github.com/iced-rs/iced/pull/3000) is **merged** into master (June 2025). Uses `hot` feature flag + `subsecond` / `cargo-hot`. Not in 0.14 release — requires iced from git or waiting for 0.15 / 1.0.

**Blocked on:** iced 0.15 or 1.0 release.

### Scrollbar minimum thumb height (upstream PR candidate)

With 20k+ items in a virtualized list, iced's scroll thumb shrinks to its hardcoded 2px minimum and becomes visually imperceptible / ungrabbable. The constant lives at [iced_widget-0.14.2/src/scrollable.rs:1998](https://github.com/iced-rs/iced/blob/0.14/widget/src/scrollable.rs#L1998) (and the horizontal mirror around line 2068):

```rust
let scroller_height = (scrollbar_bounds.height * ratio).max(2.0);
```

`Scrollbar` exposes `width`, `margin`, `scroller_width`, `alignment`, `spacing` — but not `min_scroller_height` / `min_scroller_length`. `scrollable::Scroller` style only has `background` and `border`, and borders are drawn inside bounds so they can't visually enlarge a 2px thumb.

**Upstream PR shape:**
1. Add `min_scroller_length: f32` field (default `2.0` to preserve current behavior) to `Scrollbar` struct.
2. Add a `min_scroller_length(impl Into<Pixels>) -> Self` builder method alongside the existing `width`, `margin`, `scroller_width`.
3. Replace `.max(2.0)` with `.max(self.min_scroller_length)` in both the vertical (~line 1998) and horizontal (~line 2068) branches of the layout pass.

Total patch: ~5 lines + doc. No behavior change for existing users (default stays at 2.0). Fully backward-compatible.

**Workaround until this lands:** either vendor `scrollable.rs` locally (~2400 LOC — we already fork `drop_down.rs` so precedent exists, but it's heavy) or overlay a custom thumb next to a `Scrollbar::hidden()` scrollable (~150–250 LOC new widget in `components::`, drag sync via `scrollable::scroll_to` operation).

**Decision:** wait on upstream. If the PR is rejected or drags, revisit with the overlay approach — the fork is the worst of both worlds.

### `Arc<[CipherListView]>` micro-optimization

Today `VaultView.items.all` and `.cached` hold `Vec<Arc<CipherListView>>`, paying a refcount bump per item on every clone/filter pass. The whole vec could share one allocation: `Arc<[CipherListView]>`, with filter/search operating on indices into the shared slice (or returning a fresh `Arc<[CipherListView]>` built from the filtered subset). The `VaultMessage::ListLoaded` variant would simplify to `Result<Arc<[CipherListView]>, String>`.

`Vec<T> → Arc<[T]>` doesn't require `T: Clone`, so `CipherListView: !Clone` isn't a blocker.

**Blocker:** do **not** do this until the Tier 1 load-test account exists and we can measure before/after. Premature optimization otherwise.

### LoginView per-AuthPage field grouping

From the audit in the last refactor round: `LoginView` has field clusters per auth page (Unlock / LoginEmail / LoginPassword / ServerSelector) that could be extracted into sub-structs following the pattern used for `App` and `VaultView`. The win is real but the churn is high — touches 4 sub-widget files + every `LoginView::update` match arm.

**Blocker:** defer until we're touching `LoginView` for another reason anyway (e.g. the Registration view in Tier 2, which will make the clusters even clearer).

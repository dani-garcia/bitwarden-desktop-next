# TODO

## Contents

- [Top Priority: Investigate View Encapsulation](#top-priority-investigate-view-encapsulation)
- [Next Up](#next-up)
- [UI Polish](#ui-polish)
- [Auth Flow](#auth-flow)
- [Secure Text Input](#secure-text-input)
- [Functionality](#functionality)
- [Virtual List / Lazy Scrolling](#virtual-list--lazy-scrolling)
- [Testing](#testing)
- [Developer Experience](#developer-experience)
- [Multi-Window Support](#multi-window-support)
- [Toast API Review](#toast-api-review)

---

## Top Priority: Investigate View Encapsulation

**Current pain.** `crates/desktop/src/app.rs` has:

- a **13-variant `Message` enum** where every async callback, every child message category, and every cross-cutting concern is centralized in one place (`Login(LoginMessage)`, `Vault(VaultMessage)`, `TitleBar(TitleBarMessage)`, `UnlockCompleted`, `VaultListLoaded`, `CipherDetailLoaded`, `CloseToast`, `PollNativeMenu`, `KeyPressed`, `WindowOpened`, `GotRawId`, `SystemThemeChanged`);
- a **~220-line `update()` function** that's one giant `match` over those variants, mixing screen routing, async task dispatch, toast pushing, menu handling, theme sync, and window chrome;
- **29 `Message::` touch-sites** across `app.rs` just to wire up existing views.

Every new feature today means: (1) add a `Message` variant, (2) add a match arm in `update()`, (3) often add a field on `App`, (4) thread the call through from the view. That's a lot of shared-file churn and it's the main thing that worries us about scaling to multiple views / multiple teams.

### What we already know

- **`iced::Component` is dead.** It was deprecated in iced 0.13 with this rationale: *"components introduce encapsulated state and hamper the use of a single source of truth. Instead, leverage the Elm Architecture directly, or implement a custom widget"*. So the "wrap each view in a Component" escape hatch isn't on the table — the iced maintainers explicitly decided against it.
- **Custom widgets are still fine.** That's the path we already used for `components::toast::Manager` and `components::drop_down::DropDown`. Custom widgets are appropriate when the encapsulation boundary is *visual + behavioral* (a reusable thing with its own private state like fade timers), not *organizational* (one team owns the settings screen).
- **Rust + Elm has an inherent tension.** [héctor on Zulip](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Async.20component.20updates/with/216384680) is explicit that the borrow checker can't let views mutate shared state concurrently, so forcing continuations through `Message` variants is unavoidable. The centralized message dispatch isn't a flaw of iced — it's a property of doing Elm in Rust.
- **Our partial decentralization is already on-pattern.** `LoginView`/`VaultView`/`TitleBarState` each own their local state and return `Vec<Action>` from `update()`. App is a thin dispatcher that translates actions to cross-cutting effects (screen switch, async tasks, toast pushes). This *is* the Elm Architecture's canonical "helper functions on sub-models" approach. We didn't invent it accidentally — but we may not be applying it optimally.

### Reframed investigation: how does the Elm Architecture *correctly* apply at scale?

The question is no longer "how do we escape centralized state?" — it's "**what's the canonical Elm Architecture pattern that scales to large apps, and where are we deviating from it without realizing?**"

Concrete things to learn:

- **Read the Elm guide carefully**, especially [The Elm Architecture](https://guide.elm-lang.org/architecture/) (the canonical Model/View/Update primer), [the structure chapter](https://guide.elm-lang.org/webapps/structure.html), and [larger applications](https://guide.elm-lang.org/webapps/structure.html#larger-applications). The official position is that "modules" + "helper functions on sub-models" scale fine and that anyone reaching for components is using the wrong frame. Verify what their concrete recommendation is for our class of problem (multiple full screens, async task results, cross-cutting concerns like toasts).
- **Read the unofficial iced guide** at <https://jl710.github.io/iced-guide/>. Much more detailed than the official iced docs, covers structuring patterns, custom widgets, async/Task usage, and multi-screen apps. Likely the best single resource for "how to actually build a non-trivial iced app". Source repo at <https://github.com/JL710/iced-guide> — worth cloning as a submodule alongside the iced apps below so the markdown source is available locally for grep / Claude parsing.
- **Real-world iced apps at scale.** Clone these as git submodules (mirroring how `clients/` already pulls in the official Bitwarden app for reference) so we can grep through them and see how they organize state:
  - **Halloy** — IRC client, multi-pane, multi-server. Biggest iced app in the wild we know of. <https://github.com/squidowl/halloy>
  - **Gauntlet** — cross-platform launcher with a plugin system, full app with settings, lists, detail views. <https://github.com/project-gauntlet/gauntlet>
  - **Cosmic Files / Cosmic Settings / Cosmic Edit** (System76) — full desktop apps in iced, with settings panels, file pickers, etc.
  - **Sniffnet** (network monitor) — has multiple "pages"

  For each: count their top-level message variants, see how their `update()` is structured, look at how they wire async results back into sub-screens, and how they handle cross-cutting concerns (theming, notifications, modals). Document one or two patterns that look promising in `docs/decisions.md`.
- **Real-world Elm apps at scale** ([elm-spa](https://www.elm-spa.dev/), [RealWorld example](https://github.com/rtfeldman/elm-spa-example), [Lamdera](https://lamdera.com/)). They handle multi-team ownership with "pages" that own their `Model`/`Msg`/`update`/`view`, glued by a top-level router. This is what we already do — but their routing layer may have idioms we're missing (e.g. how they handle messages targeted at a page that isn't currently mounted).
- **The async-callback problem specifically.** `UnlockCompleted`/`VaultListLoaded`/`CipherDetailLoaded` are top-level only because `Task::perform` returns `Task<Message>`, not a scoped sub-message. Is there a `Task::map` / `Message::Vault(VaultMessage::ListLoaded(...))` wrapper pattern that would let us push these inside `VaultMessage` instead of having them at the App level? If so, the App-level enum collapses to roughly just `Login(_)`, `Vault(_)`, `TitleBar(_)`, `Window(_)`, `System(_)` plus a couple of orchestration variants — which is a *lot* less daunting to look at.
- **Helper functions on the App model.** Even if we keep one big `Message` enum, the 220-line `update()` body could be split into `fn handle_login_action(&mut self, action) -> Task`, `fn handle_vault_action(...)`, `fn handle_unlock_completed(...)` etc. — each taking `&mut self`. The match arms become one-liners that delegate. This is the Elm-recommended fix and it's purely mechanical. Worth doing as the *first* concrete step regardless of what the bigger investigation concludes.
- **Sub-enum pattern taken further.** Today the `Login`/`Vault`/`TitleBar` wrapper variants already isolate their child messages. Apply the same trick to the orphan callbacks: `Message::SdkResponse(SdkResponse)` containing `Unlock(...)`, `VaultListLoaded(...)`, `CipherDetailLoaded(...)`. Same trick for window/system stuff: `Message::Window(WindowMessage)` containing `Opened`, `GotRawId`, `KeyPressed`, `PollNativeMenu`, `SystemThemeChanged`. The total information stays the same but the top-level enum shrinks from 13 to ~6, and grouping by ownership makes the match readable.

### Concrete deliverables

1. **A short writeup** (in `docs/decisions.md`, "View Architecture" section) summarizing the canonical Elm-in-iced pattern and why we picked it. Killing the temptation to re-investigate this every six months.
2. **Refactored `app.rs`** that reflects whatever the investigation concludes. At minimum: split `update()` into `handle_*` helper methods so the match body is one-liners. At most: re-group the `Message` enum into sub-enums and move the SDK callbacks into `VaultMessage`/`LoginMessage`.
3. **A "how to add a new view" page** in `docs/architecture.md`: the canonical recipe for adding (say) a Settings or Generator screen, end-to-end, with no shared-file churn surprises. Future contributors should be able to follow it without reading any other doc.

---

## Next Up

- **Tray icon** — Use `tray-icon` crate (sister to `muda`, same raw window handle approach). Should show Bitwarden shield icon, right-click context menu with Lock/Quit. Iced PR https://github.com/iced-rs/iced/pull/3021 adds native tray support but is still **open** (targeting 1.0), so use `tray-icon` crate directly for now.
- **Avatar color auto-generation** — Generate avatar background color from username/email hash (like the official app does) instead of using a fixed color.
- **Unlock loading indicator** — While `ClientManager::unlock(...)` is decrypting the user key and the subsequent `list_ciphers(...)` decrypts the vault, the UI sits frozen on the unlock screen with no feedback. Replace the "Unlock" button with a greyed-out spinner (or disable + show a spinner next to it) from the moment `LoginAction::Unlock(...)` fires until either `Message::VaultListLoaded` arrives with `Ok` or `Message::UnlockCompleted` arrives with `Err`. Needs a small `unlock_in_progress: bool` on `LoginView` (or on App) and a spinner widget — probably `iced::widget::progress_bar` in indeterminate mode or a custom rotating icon. While in-progress: disable the input, hide the close-other-method links, show the spinner where the button was.
- **Load-test account** — Add a third mock user to `tools/fake-data/src/main.rs` with ~20k ciphers (mix of logins, cards, notes; random names from a small word bank). Regenerated JSON will be much bigger but still embeddable via `include_bytes!`. This unblocks real testing of the Virtual List / Lazy Scrolling work below — right now we only have ~20 items per user so layout/render perf looks fine.
- **`tracing` + `tracing_subscriber` for logging** — Replace the handful of `eprintln!` calls in `app.rs` (unlock failures, vault list loading, cipher decrypt errors) with structured `tracing::info!` / `warn!` / `error!` calls. Add `tracing` and `tracing_subscriber` as workspace deps, initialize a subscriber in `main.rs` (env-filter by default, e.g. `RUST_LOG=bitwarden_desktop_next=debug`). The SDK itself already emits `tracing` spans (`#[tracing::instrument]` on `initialize_user_crypto` and others) — once a subscriber is installed, those become visible for free, which will be valuable when debugging the unlock flow and the load-test scenario.
- **Collapse `Vec<Arc<CipherListView>>` to `Arc<[CipherListView]>` (or `Rc<[CipherListView]>`)** — Today `VaultView::all_items` and `cached_items` each hold `Vec<Arc<CipherListView>>`, paying a refcount bump per item on every clone/filter pass. Since the full vault list is always cloned/filtered together (we never hand out individual `Arc<CipherListView>`s to different owners), the whole vec could be shared as one allocation: `Arc<[CipherListView]>`. Filter and search would then operate on indices into the shared slice, or return a fresh `Arc<[CipherListView]>` built from the filtered subset. The `Message::VaultListLoaded` variant simplifies to `Result<Arc<[CipherListView]>, String>`. Need to verify `CipherListView: !Clone` doesn't block the slice construction (we'd build a `Vec<CipherListView>` first and call `.into()` — should work since `Vec<T> -> Arc<[T]>` doesn't require `T: Clone`). Measure before/after with the 20k-cipher load-test account.

---

## UI Polish

- Sidebar: indented tree hierarchy (Vault > All vaults > My vault)
- Sidebar expand/collapse animation (Iced 0.14 has no built-in layout transitions; needs `Subscription` tick + interpolated width)
- TOTP circular timer in detail pane (currently placeholder text)
- Account switcher dropdown: visual update to match 2025 Figma (Lock/Logout buttons, Options section)
- SVG logo antialiasing — Iced's resvg rasterizer doesn't match browser quality; consider pre-rasterized PNG

---

## Auth Flow

- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields.
- **Master password hint request** — "Get master password hint" link on login password screen. Sends hint request to server.
- **Self-hosted server URL modal** — Server selector "Self-hosted" option should open a modal to input custom server URL.
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

---

## Secure Text Input

- **Problem** — Iced's `text_input` uses a standard `String` internally. When the user types a master password, that string lives in heap memory and isn't scrubbed on drop. If the memory is released and reallocated, the password could linger in the process address space. See [Zulip discussion](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Secure.20text_input/with/221416543).
- **Practical impact is debatable** — As discussed in that thread, an attacker with enough access to probe process memory likely has easier vectors (keyloggers, `/proc/mem`, input subsystem ring buffers). Memory scrubbing is defense-in-depth, not a primary control. The consensus is it's worth doing as best-effort but shouldn't be over-invested in.
- **Our SDK already helps** — The Bitwarden SDK sets up a zeroizing memory allocator, so secrets handled through SDK types are already scrubbed. The gap is specifically the iced `text_input` widget's internal `Value` struct which holds the password string before it reaches the SDK.
- **Possible approaches**: (1) Fork `text_input`/`value.rs` to use `secrecy::SecretString` or `zeroize::Zeroizing<String>` as the backing store — most thorough but maintenance burden on iced upgrades. (2) Minimize exposure window — copy the password out of the text input into a zeroizing type immediately on submit, then clear the input field (already done for UX reasons). (3) Accept the gap as low-risk given the SDK's allocator and focus security effort elsewhere.
- **Decision**: low priority. Approach (2) is essentially free and we should verify we're doing it. Approach (1) only if compliance requires it.

---

## Functionality

- Wire copy buttons in detail pane to clipboard (arboard crate or iced clipboard API)
- Wire edit/delete buttons in detail pane (currently stubs)
- Handle tray icon click to show/hide window
- "Unlock with Windows Hello" button (future, needs keyring/biometric integration)

---

## Virtual List / Lazy Scrolling

A user might have 20k+ vault items. Currently `item_list.rs` builds a `Column` with all items. The bottleneck is **layout and widget tree construction in `view()`**, not rendering — iced's `Column::draw()` already culls offscreen children via `layout.bounds().intersects(viewport)`, but `Column::layout()` and `Column::update()` iterate ALL children regardless.

### What iced 0.14 provides

- **`lazy(dependency, |dep| view)`** (`iced_widget`, `lazy` feature) — Hashes `dependency`; if unchanged, reuses the cached `Element` tree without re-calling the closure. Useful as a **supplement** (skip rebuilds when scroll window + data haven't changed), but does NOT reduce the number of elements — when the hash changes, all 20k elements are rebuilt at once.
- **`scrollable(...).on_scroll(|viewport| Msg)`** — Fires on every scroll position change. `Viewport` gives `absolute_offset().y` (pixel offset), `bounds().height` (visible area), and `content_bounds().height` (total content). This is the key API for computing which rows are visible.
- **`Sensor` widget** (`iced_widget::sensor`) — Fires `on_show`/`on_hide` when content enters/exits viewport, with `.anticipate(pixels)` for prefetching. However, still requires creating one `Sensor` per item in the tree, so not suitable for 20k items where tree construction is the bottleneck.
- **`table` widget** (`iced_widget::table`) — Built-in table, but eagerly constructs ALL cells in `Table::new()` (no virtualization). With 20k rows × 4 columns = 80k Elements. Not suitable without pre-slicing the data.

### What doesn't exist

- **No community crate** — No `iced_virtual_list`, `iced-virtual-scroll`, or equivalent exists on crates.io or GitHub. The only virtual list crates found are for egui.
- **No upstream implementation** — [iced#160](https://github.com/iced-rs/iced/issues/160) (`InfiniteList` widget) is open since Jan 2020, assigned to milestone 1.0, but no implementation merged.
- **No iced example** for virtual/lazy scrolling exists in the repo.

### Community-validated approach (iced Discourse)

The recommended approach from iced contributors ([discourse thread](https://discourse.iced.rs/t/virtualized-lists/1190)) is **DIY viewport-based windowing**:

1. Store `scroll_offset: f32` and `viewport_height: f32` in `VaultView` state.
2. Use `scrollable(...).on_scroll(|viewport| VaultMessage::Scrolled(viewport))` to track offset.
3. In `update()`, compute visible window: `first = (offset_y / ROW_HEIGHT) as usize`, `count = (viewport_height / ROW_HEIGHT) as usize + OVERSCAN_BUFFER`.
4. In `view()`, only build `Element`s for `items[first..first+count]`.
5. Use `Space::with_height(first * ROW_HEIGHT)` before and `Space::with_height((total - first - count) * ROW_HEIGHT)` after the visible items to maintain correct scroll thumb size and position.
6. Wrap in `lazy((first, count, data_version), ...)` to skip rebuilding when the visible window hasn't changed.

**Requires:** fixed row height (uniform across all items). This is already the case for our vault item rows.

### Implementation plan

- Add `scroll_offset`, `viewport_height`, `visible_range: Range<usize>` fields to `VaultView`.
- Add `VaultMessage::ListScrolled(scrollable::Viewport)` variant.
- Modify `item_list.rs` to accept a slice + spacer heights instead of the full item list.
- Wrap the item list `Column` in `lazy((visible_range, data_version), ...)` for frame-to-frame caching.

---

## Testing

- **Unit tests for pure logic** — `VaultView::filtered_items()`, `Shortcut::matches()`, `EnabledWhen::check()`, view `update()` state machines (message in → actions out). Standard `#[test]`, no framework needed.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test = "0.14"` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

---

## Developer Experience

- **Hot reloading** — Iced PR https://github.com/iced-rs/iced/pull/3000 is **merged** into master (June 2025). Uses `hot` feature flag + `subsecond`/`cargo-hot`. Not in iced 0.14 release yet — requires iced from git or waiting for 0.15/1.0.

---

## Multi-Window Support

- **Use `iced::daemon` instead of `iced::application`** — Per-window views are NOT supported by `iced::application` (the `view()` function has no `window::Id` parameter). The official way to get per-window views is `iced::daemon(boot, update, view)`, where `view(&self, window::Id) -> Element` receives the window ID and can return different content per window. Despite the name, `daemon` is not about background processes — it's just the iced entry point that gives full control over windows (including running with no windows at all). See [Zulip discussion](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/.E2.9C.94.20Support.20for.20per-window.20views/with/577450159).
- **Migration from `application` to `daemon`** — Switching to `daemon` means: (1) no automatic main window — `boot()` must call `window::open()` and store the returned `window::Id`, (2) `view()` gains a `window::Id` parameter and branches on it, (3) app must subscribe to `window::close_events()` and call `iced::exit()` when the main window closes, otherwise the process keeps running. Minimal example:
  ```rust
  fn main() -> Result {
      iced::daemon(App::boot, App::update, App::view)
          .subscription(App::subscription)
          .run()
  }
  // boot() calls window::open(Default::default()) and stores main_window: window::Id
  // view(&self, id: window::Id) branches on id == self.main_window vs child windows
  // subscription() listens to window::close_events(), exits when main window closes
  ```
- **Window close event handling is critical** — With `daemon`, if the user closes the main window but child windows are still open (or even if they're not), the process keeps running invisibly. Must subscribe to `window::close_events()` and: (1) call `iced::exit()` when the main window closes (kills everything), (2) for child windows, remove their entry from the `HashMap<window::Id, _>` so we stop tracking them. If we don't handle this, the app becomes an orphan process with no visible window and no way to quit except task manager.
- **Reference example**: https://github.com/iced-rs/iced/blob/master/examples/multi_window/src/main.rs — uses `HashMap<window::Id, Window>` to track per-window state.

### Modal as alternative to popup windows

For many use cases (item editor, password generator, settings forms, confirmations), an **in-window modal overlay** may be preferable to spawning a separate OS window. Iced's official modal example (https://github.com/iced-rs/iced/blob/master/examples/modal/src/main.rs) shows a clean pattern:

- **Implementation**: a `modal(base, content, on_blur)` helper function using `stack!` to layer: base content → `opaque()` dark overlay (`Color::BLACK` at 0.8 alpha) → centered modal content. Uses `mouse_area` on the overlay to detect click-outside and fire the `on_blur` message.
- **Dismiss**: click outside triggers `on_blur` message via `mouse_area`. Escape key handled separately in `update()` by matching `keyboard::Event::KeyPressed { key: Named(Escape) }`.
- **Conditional rendering**: `if self.show_modal { modal(content, dialog, Msg::Hide) } else { content.into() }`.
- **Pros over popup windows**: no daemon migration needed, no window lifecycle management, modal stays in context of the main app, works on all platforms identically, simpler state management.
- **Cons**: can't drag a modal out of the main window, can't view the modal side-by-side with the main content, limited to one overlay level (iced constraint — we already deal with this for our DropDown fork).

### When to use which

- **Modal**: item editor, password generator, settings forms, confirmations, "about" dialog — anything where the user acts and returns to the main view.
- **Popup window**: use cases where the user needs to see/interact with both windows simultaneously (e.g. comparing two vault items side by side). Requires daemon migration.

---

## Toast API Review

The toast system is live in [components/toast.rs](../crates/desktop/src/components/toast.rs): `Manager` widget wrapping the app content, overlay with fade in/out, progress bar, hover-pause, severity-specific icon + color. Call sites today only use `Toast::warning(body, title)` from the PIN/biometrics stubs in `app.rs`.

Before we grow more call sites (unlock failure, copy-to-clipboard, sync errors, etc.) we should validate the API:

- **Is the `Toast::info/success/warning/error(body, title)` surface the right shape?** The `title: Option<&str>` slot is a common pain point — callers have to pass `None` in the default case, which is a tiny wart on every call. Alternatives: a builder (`Toast::warning("body").with_title("Title")`), default + overloaded methods (`Toast::warning(body)` defaults, `Toast::warning_titled(body, title)` overrides), or drop the title entirely and only expose `body`. Pick whichever reads cleanest at call sites.
- **Ergonomics from anywhere in the app.** Right now `App::push_toast(Toast)` is the only entry point, and it's a private method on `App`. Any view that wants to emit a toast has to return an action enum variant that App handles. Consider: should toasts be part of the `Vec<Action>` returned by views (cleanest), a shared `Arc<Mutex<ToastQueue>>` passed down, or a dedicated `ToastSink` trait? Validate with at least two hypothetical call sites outside `app.rs`.
- **Is `Manager` the right wrapper level?** It currently wraps the entire `column![title_bar, page]`. Does that cover all the cases we'll need, or do we want per-view toast managers?
- **What should the `Toast` struct even carry?** Today it's `{ title, body, status }`. Future callers may want an action button ("Undo"), an icon override, a sticky flag (no auto-dismiss), or a custom timeout. Plan the schema before we have ten call sites to migrate.
- **Document the final design** in `docs/decisions.md` once the shape is locked in.

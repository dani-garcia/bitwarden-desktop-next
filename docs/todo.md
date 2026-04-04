# TODO

## Next Up

- **Tray icon** — Use `tray-icon` crate (sister to `muda`, same raw window handle approach). Should show Bitwarden shield icon, right-click context menu with Lock/Quit. Iced PR https://github.com/iced-rs/iced/pull/3021 adds native tray support but is still **open** (targeting 1.0), so use `tray-icon` crate directly for now.
- **Avatar color auto-generation** — Generate avatar background color from username/email hash (like the official app does) instead of using a fixed color.

## UI Polish

- Sidebar: indented tree hierarchy (Vault > All vaults > My vault)
- Sidebar expand/collapse animation (Iced 0.14 has no built-in layout transitions; needs `Subscription` tick + interpolated width)
- TOTP circular timer in detail pane (currently placeholder text)
- Account switcher dropdown: visual update to match 2025 Figma (Lock/Logout buttons, Options section)
- SVG logo antialiasing — Iced's resvg rasterizer doesn't match browser quality; consider pre-rasterized PNG

## Auth Flow

- **Registration view** — "Create account" link on login email screen navigates here. Needs email, password, hint fields.
- **Master password hint request** — "Get master password hint" link on login password screen. Sends hint request to server.
- **Self-hosted server URL modal** — Server selector "Self-hosted" option should open a modal to input custom server URL.
- **SSO login flow** — "Use single sign-on" button on login email screen. Needs SSO provider selection + browser redirect.

## Secure Text Input

- **Problem** — Iced's `text_input` uses a standard `String` internally. When the user types a master password, that string lives in heap memory and isn't scrubbed on drop. If the memory is released and reallocated, the password could linger in the process address space. See [Zulip discussion](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Secure.20text_input/with/221416543).
- **Practical impact is debatable** — As discussed in that thread, an attacker with enough access to probe process memory likely has easier vectors (keyloggers, `/proc/mem`, input subsystem ring buffers). Memory scrubbing is defense-in-depth, not a primary control. The consensus is it's worth doing as best-effort but shouldn't be over-invested in.
- **Our SDK already helps** — The Bitwarden SDK sets up a zeroizing memory allocator, so secrets handled through SDK types are already scrubbed. The gap is specifically the iced `text_input` widget's internal `Value` struct which holds the password string before it reaches the SDK.
- **Possible approaches**: (1) Fork `text_input`/`value.rs` to use `secrecy::SecretString` or `zeroize::Zeroizing<String>` as the backing store — most thorough but maintenance burden on iced upgrades. (2) Minimize exposure window — copy the password out of the text input into a zeroizing type immediately on submit, then clear the input field (already done for UX reasons). (3) Accept the gap as low-risk given the SDK's allocator and focus security effort elsewhere.
- **Decision**: low priority. Approach (2) is essentially free and we should verify we're doing it. Approach (1) only if compliance requires it.

## Functionality

- Wire copy buttons in detail pane to clipboard (arboard crate or iced clipboard API)
- Wire edit/delete buttons in detail pane (currently stubs)
- Handle tray icon click to show/hide window
- "Unlock with Windows Hello" button (future, needs keyring/biometric integration)

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

## Testing

- **Unit tests for pure logic** — `VaultView::filtered_items()`, `Shortcut::matches()`, `EnabledWhen::check()`, view `update()` state machines (message in → actions out). Standard `#[test]`, no framework needed.
- **Integration tests with `iced_test`** — headless simulator for click/type/find workflows. `iced_aw` 0.13 has extensive examples in `tests/` to reference. Add `iced_test = "0.14"` as dev-dependency.
- **Snapshot tests** — optional, for catching visual regressions in theme/layout changes.

## Architecture: View Encapsulation & Team Ownership

The Elm architecture (which iced follows) has no concept of "components" — the entire app is one `update()` + one `view()`, with state and messages defined at the top level. This conflicts with our goal of **each view being owned by a separate team**, where a team can modify their view without touching other teams' files or the root `App` struct.

### The tension

- **Elm's position** ([guide](https://guide.elm-lang.org/webapps/structure.html#components)): thinking in components is discouraged. Instead, scale by splitting `update` and `view` into helper functions that operate on subsets of the model. Messages stay flat, state stays centralized.
- **héctor (iced maintainer)** in [Zulip](https://iced.zulipchat.com/#narrow/channel/213316-discussions/topic/Async.20component.20updates/with/216384680): the scaling problem (needing to break `a(); modify self; b()` into separate message variants for each continuation) is inherent to concurrent programming in Rust, not a flaw of Elm. Rust won't let you mutably reference part of your state in the background while using it in the foreground.
- **Our current approach**: already partially decentralized — `LoginView`, `VaultView`, `TitleBarState` each own their state and return `Vec<Action>` from `update()`. App is a thin dispatcher. This is the "helper functions on sub-models" approach Elm recommends, not true components.

### What we need to research

- **How far can the current pattern scale?** — Right now views return action enums (`LoginAction`, `VaultAction`) that App processes. Adding a new view means: (1) new view struct + message enum + action enum, (2) new `Message` variant in App, (3) new arm in App's `update()` and `view()`. Steps 2-3 touch shared files. Can we reduce that coupling?
- **iced `component` / `Component` trait** — iced had an experimental `Component` trait (see `iced_lazy::Component` in older versions). It allowed self-contained widgets with their own internal state and messages, only emitting output messages to the parent. Research whether this still exists in 0.14, whether it was removed/replaced, and whether it's suitable for full views (not just small widgets).
- **Message routing via trait objects or registry** — Could views register themselves so App doesn't need to know about each one? e.g. `Box<dyn View>` with `update(&mut self, msg) -> Vec<Box<dyn Action>>`. Evaluate ergonomics vs type safety trade-off.
- **Elm community patterns for large apps** — Look at how large Elm apps (e.g. elm-spa, RealWorld) handle multi-team ownership. The usual answer is "pages" with their own `Model`/`Msg`/`update`/`view`, glued together by a top-level router. This is close to what we have.
- **Other iced apps at scale** — Find open-source iced apps with many views/screens and see how they structure state and messages. Halloy (IRC client), Cosmic desktop apps (System76), Sniffnet.

### Goal

A pattern where adding a new view (e.g. "Settings", "Generator") requires: (1) creating files in a new `views/settings/` directory, (2) a minimal one-line registration or import in a shared file — NOT modifying App's `update()` match arms, `Message` enum variants, or `view()` branches by hand. Each view team should be able to work independently.

## Developer Experience

- **Hot reloading** — Iced PR https://github.com/iced-rs/iced/pull/3000 is **merged** into master (June 2025). Uses `hot` feature flag + `subsecond`/`cargo-hot`. Not in iced 0.14 release yet — requires iced from git or waiting for 0.15/1.0.

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

## SDK Integration

- **Fill mock clients with fake data** — `sdk.rs` has `mock_personal_client()` and `mock_work_client()` with empty `MemoryRepo`s. Populate them with realistic `Cipher` and `Folder` data (matching what `mock.rs` currently provides) so the vault view can read from the SDK instead of flat structs.
- **Wire SDK to application logic** — Replace `mock::mock_users()` and the flat `UserSession.vault_items` with data sourced from `ClientManager`. The vault view should read cipher/folder data through `PasswordManagerClient.vault()` rather than the current `CipherItem` structs. This involves updating `App`, `refresh_cache()`, and the vault view to use SDK types.

## Toast Notifications

- **Research existing toast/notification widgets** — Before building a custom implementation, evaluate:
  - Iced's own toast example: https://github.com/iced-rs/iced/blob/master/examples/toast/src/main.rs
  - `iced-toasts` crate: https://github.com/Gomango999/iced-toasts/tree/main
  - Check if `iced_aw` has any notification/toast widget
  - Determine which approach fits best (overlay-based, stacked, timed auto-dismiss, action buttons)
- **Implement toast system** — Needed for user feedback on actions like copy-to-clipboard, unlock success/failure, network errors, sync status. Should support multiple concurrent toasts, auto-dismiss with timeout, and different severity levels (info, success, error).

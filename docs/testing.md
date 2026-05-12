# Testing

Notes on how tests are organised in this crate, the four layers, the helpers
in [`crate::test_support`](../crates/desktop/src/test_support/mod.rs), and
when each layer pays off.

## Layers

Tests fall into four layers, in increasing cost and decreasing per-test
coverage. Pick the lowest layer that exercises the thing you care about.

### Layer A — Pure logic

Free functions that take inputs and return outputs with no `App`/`View`
involvement: filter predicates, validators, format mergers, URI parsers.

```rust
#[test]
fn https_url_is_accepted() {
    assert_eq!(
        validate_self_hosted_url("https://vault.example.com"),
        SelfHostedUrl::Accepted("https://vault.example.com".into()),
    );
}
```

No setup needed — plain `#[test]`. Examples:
[`login::validate_self_hosted_url`](../crates/desktop/src/views/login/mod.rs),
[`vault::filter_items`](../crates/desktop/src/views/vault/update/list.rs),
[`import::all_formats`](../crates/desktop/src/views/import/mod.rs),
[`test_support::fixtures`](../crates/desktop/src/test_support/fixtures.rs).

### Layer B — View update unit tests

Drive a view's `update(msg, ctx)` directly, then assert on the returned
`Outcome` and on view state. Use this for state-machine logic, validation
rules, stale-uid guards, and `Outcome::Event` emission.

```rust
#[tokio::test(flavor = "current_thread")]
async fn submit_with_empty_name_returns_none() {
    let mut view = NewFolderView::new();
    view.run(NewFolderMessage::Submit).await.expect_none();
    assert!(!view.saving);
}

#[tokio::test(flavor = "current_thread")]
async fn name_changed_is_ignored_while_saving() {
    let mut view = NewFolderView::new();
    view.open();
    // Pass an array of messages for setup; the Outcome of the last one is
    // returned and typically dropped on the setup path.
    view.run([
        NewFolderMessage::NameChanged("First".into()),
        NewFolderMessage::Submit,
    ])
    .await;
    // ...assertion against the message under test...
}
```

All Layer B tests need `#[tokio::test(flavor = "current_thread")]` because
[`App::test`] spawns a [`SessionTimeout`] driver task and
[`FaviconService`] captures a runtime handle. The `view.run(...)` method
(an extension on [`View`] via `ViewTestExt`) accepts either a single
message or `[Msg; N]`; it's declared `async` even though the body is sync
as a compile-time signal. Calling `view.run(msg).expect_none()` from a
plain `#[test]` produces `no method expect_none on Future`; a bare
`view.run([..]);` call without `.await` fires the `unused_must_use`
warning. `App::test()` itself also panics with a `#[tokio::test(...)]`-pointing
message if it ever runs outside a runtime, as a final backstop.

Helpers:
- **`ViewTestExt::run(msgs)`** — method on any [`View`] that drives one
  message or an array of messages through `update()` with a fresh
  `App::test()` context. Single message → typically chain `.expect_*()`
  on the returned `Outcome`. Array of messages → discard the returned
  `Outcome` (it's the last message's); useful for arranging state before
  the message under test. Bring into scope with `use crate::test_support::ViewTestExt;`.
- **`OutcomeExt::expect_none()` / `expect_event()` / `expect_toast()`** —
  shrinks the `match Outcome { ... }` boilerplate to a single call. Panics
  with a useful message on the wrong variant.
- **`App::test()` + `app.update_ctx()`** — for tests that need to customize
  the `UpdateCtx` (active user, sidebar filter, pre-loaded client manager).
  Mutate `App` fields directly, then call `view.update(msg, app.update_ctx())`.

Examples:
[`fingerprint_phrase::tests_update`](../crates/desktop/src/views/fingerprint_phrase.rs),
[`new_folder::tests_update`](../crates/desktop/src/views/new_folder/mod.rs),
[`settings::tests_update`](../crates/desktop/src/views/settings/mod.rs).

### Layer C — Simulator interaction tests

Render the view, drive widget events (clicks, typewrites, scrolls) against
an [`iced_test::Simulator`], drain the resulting messages, optionally feed
them back through the view's `update()`. Use this when the test cares about
widget wiring: that a button actually fires the right message, that a focus
operation hit the right `widget::Id`, that a text input handles keystrokes.

```rust
#[tokio::test(flavor = "current_thread")]
async fn close_button_emits_close() {
    test_support::init();
    let mut view = FingerprintModal::default();
    view.open_with("apple banana".into());
    test_support::settle_animations();

    let app = App::test();
    let element = view.view(&app.render_ctx_main());
    let messages = test_support::drive_element(element, |ui| {
        ui.click(fl!("menu-fingerprint-close").as_str())
            .expect("Close button");
    });

    test_support::assert_emitted(&messages, "Close",
        |m| matches!(m, FingerprintMessage::Close));
}
```

Helpers:
- **`drive_element(element, |ui| ...)`** — wraps the element in a
  `Simulator`, runs interactions, returns the drained `Vec<Message>`. The
  simulator runs no runtime, so Tasks don't execute and subscriptions
  don't fire — apply messages back through `view.update(...)` by hand.
- **`assert_emitted(&messages, "Description", matcher)`** — replaces the
  `assert!(messages.iter().any(|m| matches!(m, ...)), "expected ...")`
  boilerplate.
- **`settle_animations()`** — sleeps past the default `FadeInOut`
  duration so a modal captures its fully-opened frame rather than a
  mid-animation one. Call after `open()` and before the first render.
- **`init()`** — forces `ICED_TEST_BACKEND=tiny-skia` for reproducible
  output. Idempotent and parallel-safe.

### Layer D — Snapshot tests

Render the view through `iced_test::Simulator::snapshot(&theme)` and
compare against a stored PNG baseline. Catches visual regressions: layout
shifts, color changes, typography drift, theme breakage.

```rust
#[tokio::test(flavor = "current_thread")]
async fn fingerprint_modal() {
    let mut view = FingerprintModal::default();
    view.open_with("apple banana carrot dolphin eagle".to_owned());
    test_support::settle_animations();

    view.assert_themed_snapshots("fingerprint_modal").await;
}
```

Baselines land at `tests/snapshots/{name}_{light|dark}-tiny-skia.png`. The
first run writes the baseline; subsequent runs do an exact-byte compare.
**Delete the PNG to re-baseline a view** after an intentional visual
change.

Helpers:
- **`ViewTestExt::assert_themed_snapshots(name)`** — method on any [`View`]
  that runs the light + dark snapshot pair in one call. Each iteration
  builds a fresh `App::test()` with the theme installed and renders via
  `view.view(rctx)`. Tests with non-default App state or non-trait render
  entry points should inline the theme loop. Bring into scope with
  `use crate::test_support::ViewTestExt;`.
- **`assert_snapshot(path, &theme, element)`** — one snapshot at a single
  path. Use when you need a non-themed pair (e.g. a single specific layout
  variant).

## When to skip a test

Tests are opt-in, not mandatory:

- **Trivial setters / getters** — `fn name(&self) -> &str { &self.name }`
  doesn't earn a test. Field assignments inside an arm with no validation
  logic are usually trivial too.
- **Pure UI scaffolding without state** — a snapshot is the right tool
  if a regression risk exists, but skip if the layout is glue between
  helpers that each have their own coverage.
- **Glue handlers at the App level** — `App::handle_xxx_event` arms that
  forward to `ClientManager` + push a toast are typically thin enough
  that integration coverage (manual smoke) is fine.

Reach for a test when:
- The logic has more than one branch and the branches have observable
  state effects (validation rules, stale-uid guards, conditional event
  emission).
- A bug is plausible and would be silent at runtime (filter logic,
  format/url parsers, animation-state guards).
- The behavior is documented in a comment that says "this is non-obvious"
  — a test pins the contract so the comment can't drift.
- A visual regression would be costly to catch by eye (modal layout
  changes, theme tokens diverging) — Layer D.

## Building an `App` for tests

[`App::test`](../crates/desktop/src/test_support/mod.rs) returns a minimal
`App`:
- Synthetic main-window `WindowInfo` with `MAIN_WINDOW_SIZE`
- `ClientManager::empty()`
- `Settings::default()`
- `Screen::Loading`
- Default `SidebarState`
- Light theme
- Empty toast list, no `open_overlay`

Mutate fields directly to set up scenarios:

```rust
let mut app = App::test();
app.active_user = Some(uid);
app.sidebar.active_vault_filter = VaultFilter::Trash;
app.theme.current = AppTheme::dark();
app.cache.accounts = vec![account_entry];
let outcome = view.update(msg, app.update_ctx());
```

`App` fields are `pub(crate)` precisely so tests can mutate them. The
production layering rule (views receive `UpdateCtx`, not `&mut App`) is
unchanged — that's a runtime API surface, not a visibility constraint.

## Fixtures

[`test_support::fixtures`](../crates/desktop/src/test_support/fixtures.rs)
ships SDK domain-object builders to keep tests free of 20-field struct
literals:

- **`make_cipher(CipherSpec { name, kind, deleted, ... })`** — builds an
  `Arc<CipherListView>` with sensible defaults; override only the fields
  the test cares about.
- **`login_kind(Some("https://example.com"))`** — builds a
  `CipherListViewType::Login` with a single URI.

Add more builders here when a test needs a domain struct that isn't yet
covered. The point of the module is to make Layer B / C / D test bodies
read as scenario setup, not SDK-struct construction.

# Phase 1 — Fetch, cache, render

Goal: an unlocked user with `show_favicons = true` sees real PNG favicons
next to Login ciphers, cached on disk so they survive restarts. Failed
fetches are remembered so we don't re-attempt every session. Globe fallback
for everything we don't have.

## Deliverables

- New module [`crates/desktop/src/favicon.rs`](../../crates/desktop/src/favicon.rs).
- Per-user cache at `data/favicons/<user_id>/` (PNG files + `index.json`).
- Prefetch kicked off on every `VaultMessage::ListLoaded`.
- Render branch in [`item_list.rs`](../../crates/desktop/src/views/vault/widgets/item_list.rs).
- Globe fallback asset embedded.
- `handle_sign_out` wipes the user's cache directory.

## Step-by-step

### Step 1 — Embed the fallback asset

1. Copy `clients/apps/desktop/src/images/bwi-globe.png` to
   `assets/bwi-globe.png`.
2. Add to [`crates/desktop/src/assets.rs`](../../crates/desktop/src/assets.rs):

   ```rust
   pub const BWI_GLOBE_PNG: &[u8] = include_bytes!("../../../assets/bwi-globe.png");
   ```

3. Build; confirm the asset lives in the binary (`cargo bloat` or just size
   delta on `cargo build --release`).

**Test**: none yet — asset is just bytes in the binary.

### Step 2 — Add the `reqwest` dependency

`reqwest` is already transitive via the SDK with the same TLS stack, but we
need a direct dep to construct a `Client`.

In [`crates/desktop/Cargo.toml`](../../crates/desktop/Cargo.toml):

```toml
reqwest = { version = "0.12", default-features = false, features = ["rustls-tls"] }
```

`cargo check` should not change the lock file.

**Test**: `cargo check -p bitwarden-desktop-next` compiles cleanly.

### Step 3 — Build the service skeleton

Create `crates/desktop/src/favicon.rs` with the core types and a no-op
implementation that always returns globe fallback. Wiring first, fetching
after.

```rust
use std::{collections::HashMap, path::PathBuf, sync::Arc, time::SystemTime};
use std::sync::RwLock;
use tokio::sync::Semaphore;
use crate::state::UserId;

pub type Hostname = String;

pub struct FaviconService {
    http: reqwest::Client,
    caches: RwLock<HashMap<UserId, UserCache>>,
    fetch_budget: Arc<Semaphore>,
    resolve_icons_url: Arc<dyn Fn(&UserId) -> String + Send + Sync>,
    root_dir: PathBuf,
}

struct UserCache {
    entries: HashMap<Hostname, IconState>,
    dir: PathBuf,
    hydrated: bool,
}

#[derive(Clone)]
pub enum IconState {
    Pending,
    Found(Arc<Vec<u8>>),
    Missing { last_tried_at: SystemTime },
}

#[derive(Debug, Clone)]
pub enum FaviconMessage {
    BatchResolved { uid: UserId },
}

impl FaviconService {
    pub fn new(resolve_icons_url: Arc<dyn Fn(&UserId) -> String + Send + Sync>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .pool_max_idle_per_host(5)
            .user_agent(concat!("bitwarden-desktop-next/", env!("CARGO_PKG_VERSION")))
            .use_rustls_tls()
            .build()
            .expect("reqwest client should build");

        Self {
            http,
            caches: RwLock::new(HashMap::new()),
            fetch_budget: Arc::new(Semaphore::new(5)),
            resolve_icons_url,
            root_dir: crate::paths::data_dir().join("favicons"),
        }
    }

    pub fn get(&self, uid: &UserId, hostname: &str) -> Option<IconState> {
        self.caches.read().unwrap().get(uid)?.entries.get(hostname).cloned()
    }

    pub fn evict_user(&self, uid: &UserId) { /* fs::remove_dir_all + drop entry */ }

    pub fn prefetch(
        self: Arc<Self>,
        uid: UserId,
        hostnames: Vec<Hostname>,
    ) -> iced::Task<FaviconMessage> {
        // stub: returns Task::none() for now; real impl in step 6
        iced::Task::none()
    }
}
```

Declare `mod favicon;` in [`main.rs`](../../crates/desktop/src/main.rs).

**Test**: `cargo check` compiles.

### Step 4 — Wire into `App`

In [`crates/desktop/src/app/mod.rs`](../../crates/desktop/src/app/mod.rs):

- Add `pub(super) favicon: Arc<crate::favicon::FaviconService>` to `App`.
- In `App::new`, construct with a Phase 1 resolver:

  ```rust
  let favicon = Arc::new(crate::favicon::FaviconService::new(Arc::new(
      |_uid: &UserId| "https://icons.bitwarden.net".to_string(),
  )));
  ```

- Add a `Message::Favicon(FaviconMessage)` variant; route it in
  `App::update` — the only message today is `BatchResolved`, which forces a
  redraw by virtue of arriving.

**Test**: `cargo check` passes; app still runs; nothing visible yet.

### Step 5 — Prefetch wiring (no fetch yet)

In [`views/vault/mod.rs`](../../crates/desktop/src/views/vault/mod.rs):

- Add `VaultEvent::FaviconPrefetchRequested { uid: UserId, hostnames: Vec<Hostname> }`.
- In the `VaultMessage::ListLoaded(Ok(items))` arm: iterate Login ciphers,
  extract hostnames via the helper below, dedup, emit the event.
- Add helper `hostname_for_fetch(uri: &str) -> Option<Hostname>` (put it on
  `favicon::` module for reuse):

  ```rust
  pub fn hostname_for_fetch(uri: &str) -> Option<Hostname> {
      let parsed = url::Url::parse(uri)
          .or_else(|_| url::Url::parse(&format!("http://{uri}"))).ok()?;
      match parsed.scheme() { "http" | "https" => {}, _ => return None };
      let host = parsed.host_str()?.to_ascii_lowercase();
      if host.parse::<std::net::IpAddr>().is_ok() { return None; }
      if !host.contains('.') { return None; }
      if host.ends_with(".onion") || host.ends_with(".i2p") { return None; }
      Some(host)
  }
  ```

In [`app/handlers/vault.rs`](../../crates/desktop/src/app/handlers/vault.rs):

- Route `VaultEvent::FaviconPrefetchRequested { uid, hostnames }` to
  `self.favicon.clone().prefetch(uid, hostnames).map(Message::Favicon)`.

**Test**: `cargo check` passes; unlock + list load triggers the event; the
stub `prefetch` returns `Task::none()`; nothing visible yet. Add a
`tracing::debug!` inside the stub to confirm the uid + hostname count arrive
as expected.

### Step 6 — On-disk cache format

Define the sidecar schema:

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct IndexFile {
    version: u32,                                  // = 1
    entries: HashMap<Hostname, IndexEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "status")]
enum IndexEntry {
    Found { fetched_at: chrono::DateTime<chrono::Utc> },
    Missing { last_tried_at: chrono::DateTime<chrono::Utc> },
}
```

Implement:

- `fn path_for(uid: &UserId) -> PathBuf` → `root_dir / uid.to_string()`.
- `fn png_path(uid_dir: &Path, host: &Hostname) -> PathBuf` → `uid_dir/<hostname>.png`.
- `fn hydrate_from_disk(&self, uid: &UserId)` — set `hydrated = true` after
  first call, short-circuit thereafter. Read `index.json`; for each `Found`
  entry read the `.png` bytes into `Arc<Vec<u8>>`; for each `Missing` just
  load the timestamp. Skip entries whose PNG file is missing/unreadable.
- `fn persist_index(&self, uid: &UserId)` — write-temp-then-rename
  `index.json`.

Write path has a subtlety: we write the PNG file first, then update
`index.json`. Crash between the two is benign — orphan PNG will be
overwritten on next successful fetch or picked up by a future eviction pass.

**Test**: unit tests for hydrate / persist round-trip in a `tempdir`.

### Step 7 — Real fetch with semaphore

Implement `fetch_one`:

```rust
async fn fetch_one(
    self: Arc<Self>,
    uid: UserId,
    hostname: Hostname,
) -> (Hostname, IconState) {
    let Ok(_permit) = self.fetch_budget.clone().acquire_owned().await else {
        return (hostname, IconState::Missing { last_tried_at: SystemTime::now() });
    };
    let url = format!("{}/{}/icon.png", (self.resolve_icons_url)(&uid), hostname);
    let resp = self.http.get(&url)
        .timeout(std::time::Duration::from_secs(10))
        .send().await;
    let state = match resp {
        Ok(r) if r.status().is_success() => match r.bytes().await {
            Ok(b) => {
                let bytes = b.to_vec();
                // Best-effort disk write; a failure only affects persistence.
                let _ = self.clone().write_png(&uid, &hostname, &bytes).await;
                IconState::Found(Arc::new(bytes))
            }
            Err(_) => IconState::Missing { last_tried_at: SystemTime::now() },
        },
        _ => IconState::Missing { last_tried_at: SystemTime::now() },
    };
    (hostname, state)
}
```

Implement `prefetch` properly:

1. `hydrate_from_disk(&uid)` if the user cache isn't hydrated.
2. For each hostname: if the in-memory map has `Found` or `Missing`, skip.
   Otherwise mark `Pending` and add to the fetch list.
3. Spawn one `tokio::spawn` per fetch (the semaphore bounds concurrency —
   `tokio::spawn` gives us fire-and-forget + the task will await the
   permit).
4. Use `futures::future::join_all` or a channel to collect results, then
   update the in-memory map + persist `index.json` once per batch.
5. Return `iced::Task::perform(batch_future, |()| FaviconMessage::BatchResolved { uid })`.

Disk write helper:

```rust
async fn write_png(
    self: Arc<Self>,
    uid: &UserId,
    hostname: &Hostname,
    bytes: &[u8],
) -> std::io::Result<()> {
    let dir = self.path_for(uid);
    tokio::fs::create_dir_all(&dir).await?;
    let tmp = dir.join(format!("{hostname}.png.tmp"));
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(tmp, dir.join(format!("{hostname}.png"))).await
}
```

**Test**:

- Unlock with a user. Confirm (via `tracing`) that batch fetches complete.
- Check `data/favicons/<uid>/` contains `.png` files and `index.json`.
- Relaunch the app; confirm hydration reads the same files.
- Kill WiFi during fetch; confirm `Missing` is persisted and no retry
  happens on next launch.

### Step 8 — Rendering

Thread `&Arc<FaviconService>`, `&UserId`, and the global `show_favicons`
bool through `VaultView::view → list_content → item_list::view`. The
signature grows by three params.

In [`item_list.rs`](../../crates/desktop/src/views/vault/widgets/item_list.rs)
around the `icon_circle` construction:

```rust
let icon: Element<Message, AppTheme> = if !show_favicons {
    initial_circle(...)                         // existing code path
} else {
    match cipher.r#type {
        CipherType::Login => {
            let host = first_uri(cipher).and_then(|u| hostname_for_fetch(&u));
            match host.as_deref().and_then(|h| favicon.get(uid, h)) {
                Some(IconState::Found(bytes)) => png_in_circle(bytes.clone(), 32.0),
                _ => globe_in_circle(32.0),     // Pending / Missing / None / no URI
            }
        }
        // TODO: replace globe with per-type BWI icon for Card/Identity/Note/SshKey.
        _ => globe_in_circle(32.0),
    }
};
```

Helpers:

- `png_in_circle(bytes: Arc<Vec<u8>>, size: f32)` — `iced::widget::image(
  image::Handle::from_bytes(bytes.as_ref().clone()))` inside a 32×32
  rounded-corner container. (The `.clone()` is a one-shot `Vec<u8>` clone
  into iced's handle, unavoidable.)
- `globe_in_circle(size: f32)` — same shape but from `assets::BWI_GLOBE_PNG`.

**Test**: unlock, disable `show_favicons`, list shows initial circles.
Enable, list shows globes briefly then real favicons. Disable/re-enable
toggles without restart.

### Step 9 — Log-out cleanup

In [`app/handlers/login.rs`](../../crates/desktop/src/app/handlers/login.rs)
`handle_sign_out`:

```rust
if let Some(uid) = self.active_user {
    self.vault_view.remove_user_items(&uid);
    self.favicon.evict_user(&uid);
    self.client_manager.log_out(&uid);
}
```

`evict_user` implementation:

1. Acquire write lock on `caches`, drop the `UserCache` entry.
2. `std::fs::remove_dir_all(path_for(uid))`; log warning on failure, don't
   propagate.

**Test**: log out a user from the switcher; confirm
`data/favicons/<uid>/` is gone. Log back in (re-add via fake-data); confirm
icons re-fetch from scratch.

## Observability

- `tracing::info!` on `ClientManager`-like events: service construction,
  per-user hydrate count, batch start/end with hostname counts, per-fetch
  outcome (debug level so it's not noisy by default).
- `tracing::warn!` on any disk error (write/read/remove).
- Consider a debug command (`/dev` menu?) to print cache stats: total
  entries, Found/Missing split, disk-size. Defer unless we hit a bug.

## What this phase explicitly does not do

- No `Cache-Control` parsing — even if the response includes
  `public, max-age=N` we ignore it. All entries are treated as fresh
  forever until the user logs out. This is Phase 2.
- No encryption of the on-disk cache. Anyone with filesystem access can
  enumerate the hostnames a user has Login ciphers for. Acceptable because
  the SQLite vault at `data/<uid>.sqlite` is encrypted only in memory anyway
  — same threat model. Phase 3 addresses this.
- No fetch on `save_cipher` or detail-pane open. Only ListLoaded triggers
  fetching. A TODO is left inline at both sites.
- No per-type fallback — Card/Identity/Note/SshKey all show the globe.
  A TODO is left inline in `item_list.rs`.
- No in-session retry of `Missing` entries. The user must log out /
  relaunch to get a retry. Re-evaluated in Phase 2 alongside TTL.
- No eviction policy. Disk grows monotonically until logout. Worst case
  with the loadtest account: ~5k hosts × ~3 KB PNG = ~15 MB. Fine.

## Verification checklist

- [ ] Unlock a user with `show_favicons = true` → globes immediately, real
      icons replace them within seconds.
- [ ] Second launch → icons render from disk immediately, no visible
      loading.
- [ ] Toggling `show_favicons` off → initial-letter circles.
- [ ] Toggling back on → icons (cached, instant).
- [ ] Disconnect network, unlock fresh user → all globes, no blocking.
- [ ] Log out → `data/favicons/<uid>/` removed.
- [ ] `cargo clippy` clean.
- [ ] No visible UI stutter during a batch prefetch on the loadtest account.

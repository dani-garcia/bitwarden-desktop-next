# Phase 4 — Per-user `icons_url` from real server config

Goal: when a user signs in to `vault.acme.com` (a self-hosted server), their
favicons come from that deployment's icons service, not the cloud
default. Different users in the same session use different icons servers.

Phase 1 already structured the service around a per-user resolver; this
phase swaps the closure for a real one.

## Prerequisites

- Real login flow (the "Implement the login command" item in
  [docs/todo.md](../todo.md)) needs to land first. We need a way for the
  client to hit `/api/config` on the chosen server and receive back the
  `environment` urls.
- Alternatively: self-hosted server URL modal (Tier 2 in todo.md) lands,
  and the user manually specifies the `iconsUrl` there. Either path
  populates the same field.

## Server config shape

Matching the Angular clients ([`libs/common/src/platform/services/default-environment.service.ts:26-35`](../../clients/libs/common/src/platform/services/default-environment.service.ts#L26-L35)):

```rust
pub struct EnvironmentUrls {
    pub api: Option<String>,
    pub identity: Option<String>,
    pub icons: Option<String>,
    pub web_vault: Option<String>,
    // ...
}
```

Stored on the `UserEntry` in [`sdk.rs`](../../crates/desktop/src/sdk.rs)
alongside `server_url`:

```rust
struct UserEntry {
    // ...
    server_url: String,
    icons_url: String,      // new
    // ...
}
```

And exposed via `ClientManager`:

```rust
impl ClientManager {
    pub fn icons_url(&self, uid: &UserId) -> Option<String> {
        self.users.read().unwrap().get(uid).map(|e| e.icons_url.clone())
    }
}
```

For cloud deployments the defaults match the Angular clients:

| Region | `icons_url` |
|--|--|
| US (default) | `https://icons.bitwarden.net` |
| EU | `https://icons.bitwarden.eu` |
| Self-hosted | whatever the server's `/api/config` returns, or user-entered |

## Wiring the resolver

In [`app/mod.rs`](../../crates/desktop/src/app/mod.rs), replace the Phase 1
stub:

```rust
// Phase 1
let resolver = Arc::new(|_uid: &UserId| "https://icons.bitwarden.net".to_string());

// Phase 4
let client_manager_for_favicon = Arc::clone(&client_manager);
let resolver: Arc<dyn Fn(&UserId) -> String + Send + Sync> = Arc::new(move |uid| {
    client_manager_for_favicon
        .icons_url(uid)
        .unwrap_or_else(|| "https://icons.bitwarden.net".to_string())
});
```

Key points:

- Clone of the `Arc<ClientManager>` is captured. The closure outlives
  individual fetches but is dropped when `App` drops, so no leak.
- Fallback to the cloud US default if the user has no `icons_url` —
  handles the cold-start case where the user entry exists but config
  hasn't synced yet.
- Resolves on every fetch, not once per user. A user who re-authenticates
  against a different self-hosted server without logging out gets the
  new URL automatically.

## `MockUserMeta` update

For the dev-data flow:

```rust
#[derive(Deserialize, Clone)]
struct MockUserMeta {
    // ...
    server_url: String,
    #[serde(default = "default_icons_url")]
    icons_url: String,
    // ...
}

fn default_icons_url() -> String {
    "https://icons.bitwarden.net".to_string()
}
```

Update [`tools/fake-data/src/main.rs`](../../tools/fake-data/src/main.rs) to
emit `icons_url` alongside `server_url`. Regenerate `data/mock.json` via
`cargo run -p fake-data`.

The `#[serde(default)]` keeps existing `mock.json` files working — any
user without an `icons_url` gets the cloud default on load.

## Config sync path (real login)

When the login flow lands, the server's `/api/config` response gives us
`environment` URLs. Parse and store on `UserEntry`:

```rust
pub struct ConfigResponse {
    pub environment: EnvironmentUrls,
    // ...
}

// In the login handler, after successful auth:
let config = client.config().get().await?;
entry.icons_url = config.environment.icons
    .unwrap_or_else(|| default_icons_url_for_region(&entry.server_url));
self.client_manager.update_user(entry);
```

The SDK exposes `bitwarden_core::Client::config()` — or a near equivalent;
verify at Phase 4 planning time. If not exposed, we hit the endpoint
directly via our own `reqwest::Client` (we already have one from the
favicon service).

## Cache invalidation on config change

If a user's `icons_url` changes (rare but possible — admin moves the
self-hosted icons service, or a cloud-to-self-hosted migration), the
cached icons are still valid PNGs but were fetched from a different
origin. Options:

- **Do nothing.** Existing cached entries stay in place; next
  refresh-cycle (Phase 2) fetches from the new URL.
- **Invalidate on change.** When `icons_url` changes, call
  `favicon.evict_user(&uid)` to wipe the cache and re-populate from the
  new source.

Recommend **do nothing** — the existing icons are probably fine (same
Bitwarden-icons-server codebase in most cases), and refresh eventually
reconciles. If we see issues, add invalidation as a follow-up.

## Self-hosted URL modal

Tier 2 in `todo.md` describes a "Self-hosted server URL modal" that lets
the user enter custom URLs during login. That modal's form needs an
`icons_url` field (optional, with placeholder showing the server_url + "/icons"
pattern the old Bitwarden self-hosted deployments use).

Until that modal lands, self-hosted users default to
`https://icons.bitwarden.net` (might 404 for their actual logos but won't
crash). Acceptable for now.

## Verification checklist

- [ ] User with `icons_url = "https://icons.bitwarden.eu"` in `mock.json`
      makes requests against `icons.bitwarden.eu`. Verify with a packet
      capture or HTTP trace log.
- [ ] Two users in the same session with different `icons_url` — each
      fetches from the correct server. No cross-contamination of cached
      entries (they're already isolated per-user directory).
- [ ] User with no `icons_url` in `mock.json` defaults to
      `https://icons.bitwarden.net`.
- [ ] When the login command lands, `/api/config` response's `icons` field
      populates `entry.icons_url`. If the server omits `icons`, we fall
      back to a region default based on `server_url`.
- [ ] `cargo clippy` clean.

## What this phase explicitly does not do

- No per-user TTL override. `Cache-Control` from the new server still
  drives refresh; we don't let self-hosted admins configure "ignore
  cache for icons".
- No fallback chain. If the configured icons server is unreachable, we
  don't try `icons.bitwarden.net`. `Missing` is `Missing`.
- No reconciliation of already-cached entries when `icons_url` changes.
  See the "Cache invalidation on config change" section above.

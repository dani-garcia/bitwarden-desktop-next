# Favicon / cipher-icon implementation plan

Multi-phase rollout for showing real favicons next to Login ciphers when the
user has `show_favicons` enabled. The setting is already wired in
[handlers/settings.rs](../../crates/desktop/src/app/handlers/settings.rs) — it
persists, but no render path consumes it yet. This directory describes how we
get there.

## Phase index

| Phase | Goal | Status |
|-------|------|--------|
| [Phase 1](./phase-1.md) | Fetch + disk cache (unencrypted) + 5-concurrent fetches + globe fallback + persistent Missing tracking | Not started |
| [Phase 2](./phase-2.md) | Respect `Cache-Control: max-age`, stale-while-revalidate refresh | Not started |
| [Phase 3](./phase-3.md) | Encrypt the on-disk cache, re-evaluate SQLite as the storage backend | Not started |
| [Phase 4](./phase-4.md) | Per-user `icons_url` sourced from real server `/api/config` | Not started |

Each phase is independently shippable. Phase 1's public API was designed so
the later phases don't force changes at call sites — the resolver closure in
Phase 1 is the hook Phase 4 swaps, the `index.json` sidecar schema is
versioned so Phase 2 adds `max_age_secs` without migrating, and the
service's public surface (`get`, `prefetch`, `evict_user`) stays stable
through Phase 3 even if the storage backend changes underneath.

## Key design choices (cross-cutting)

- **Per-user isolation from day one** — each user has their own subdirectory
  under `data/favicons/<user_id>/`. Even though Phase 1 only talks to one
  icons server, the service resolves the URL per-`UserId` so a session with
  multiple users pointing at different self-hosted servers works correctly
  in Phase 4 without any restructuring.
- **Full hostname, not eTLD+1** — matches the Angular clients. `mail.google.com`
  and `google.com` get separate cache entries. Source: `libs/common/src/platform/misc/utils.ts`
  `Utils.getHostname()` calls `tldts`'s `getHostname()` which returns the
  full hostname; `getDomain()` would return eTLD+1 but the clients don't
  use it.
- **Login ciphers only** — other cipher types (Card / Identity / Note / SshKey)
  have no URI and always render the globe fallback in Phase 1. Future work
  may render per-type BWI icons instead.
- **First URI only** — matches the Angular clients' `CipherViewLikeUtils.uri`
  which returns `uris[0].uri` for `CipherView` / the flattened `uri` field
  for `CipherListView`.
- **Trusted server** — no size cap on responses, no retry on failure. One
  attempt per fetch; failure → `Missing` → globe fallback.
- **5-concurrent fetch cap** — `tokio::sync::Semaphore::new(5)` in the service.
  Every fetch acquires a permit; FIFO ordering warms visible items first.
- **Deliberate non-goals** — no in-session retry of failed fetches, no
  eviction policy (cache grows until logout), no pre-fetch on save or detail
  open. Marked as TODOs for follow-ups.

## Where to add things

- **Service**: new module `crates/desktop/src/favicon.rs`, owned by `App` as
  `Arc<FaviconService>`.
- **Render**: [crates/desktop/src/views/vault/widgets/item_list.rs](../../crates/desktop/src/views/vault/widgets/item_list.rs)
  around line 155–173 where the initial-circle is built today.
- **Prefetch trigger**: [crates/desktop/src/views/vault/mod.rs](../../crates/desktop/src/views/vault/mod.rs)
  `VaultMessage::ListLoaded(Ok(items))` arm — extract unique hostnames and
  emit `VaultEvent::FaviconPrefetchRequested { uid, hostnames }`.
- **Lifecycle**: [crates/desktop/src/app/handlers/login.rs](../../crates/desktop/src/app/handlers/login.rs)
  `handle_sign_out` — call `favicon.evict_user(&uid)` before
  `client_manager.log_out(&uid)`.
- **Fallback asset**: `assets/bwi-globe.png` — copied from
  `clients/apps/desktop/src/images/bwi-globe.png` (~500 bytes). Exposed via
  `assets::BWI_GLOBE_PNG`.

## Why not existing abstractions

- **iced has no HTTP client or blob cache.** Don't wait on upstream.
- **The SDK has no icon-service helper.** `bitwarden_core` exposes the
  client's internal `reqwest::Client` only via private fields. Using our own
  `reqwest::Client` is simpler and the TLS stack is the same (`rustls-tls`).
- **The SDK's SQLite Repository pattern is overkill for Phase 1.** See
  Phase 3 for when it becomes worth considering.

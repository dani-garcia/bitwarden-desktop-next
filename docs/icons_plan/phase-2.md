# Phase 2 — `Cache-Control: max-age` and refresh

Goal: respect the icon server's `Cache-Control: public, max-age=604800`
header so icons refresh on a schedule. Until now, Phase 1 treats every
Found/Missing entry as fresh forever.

## Deliverables

- `index.json` gains `max_age_secs` per entry; schema `version` bumps to 2.
- `IconState::Found` carries `fetched_at` and `max_age` in memory.
- On `prefetch`, stale entries are re-fetched while their existing bytes
  keep rendering (stale-while-revalidate).
- `Missing` entries get a shorter TTL so transient failures self-heal.

## Schema change

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct IndexFile {
    version: u32,                                 // = 2
    entries: HashMap<Hostname, IndexEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(tag = "status")]
enum IndexEntry {
    Found {
        fetched_at: chrono::DateTime<chrono::Utc>,
        #[serde(default = "default_max_age_secs")]
        max_age_secs: u64,
    },
    Missing {
        last_tried_at: chrono::DateTime<chrono::Utc>,
        #[serde(default = "default_missing_retry_secs")]
        retry_after_secs: u64,
    },
}

const fn default_max_age_secs() -> u64 { 7 * 24 * 3600 }      // 7 days
const fn default_missing_retry_secs() -> u64 { 6 * 3600 }      // 6 hours
```

Migration: on load, if `version == 1`, populate the missing fields with the
defaults. No file rewrite needed until the next successful write (normal
prefetch flow will rewrite the file at v2).

## In-memory state

```rust
pub enum IconState {
    Pending,
    Found {
        bytes: Arc<Vec<u8>>,
        fetched_at: SystemTime,
        max_age: Duration,
    },
    Missing {
        last_tried_at: SystemTime,
        retry_after: Duration,
    },
}

impl IconState {
    pub fn is_stale(&self, now: SystemTime) -> bool {
        match self {
            IconState::Found { fetched_at, max_age, .. } => {
                now.duration_since(*fetched_at)
                    .map(|elapsed| elapsed > *max_age)
                    .unwrap_or(false)
            }
            IconState::Missing { last_tried_at, retry_after } => {
                now.duration_since(*last_tried_at)
                    .map(|elapsed| elapsed > *retry_after)
                    .unwrap_or(false)
            }
            IconState::Pending => false,
        }
    }
}
```

## Fetch — parse the header

```rust
fn parse_max_age(headers: &reqwest::header::HeaderMap) -> Option<u64> {
    let cc = headers.get(reqwest::header::CACHE_CONTROL)?.to_str().ok()?;
    // "public, max-age=604800, immutable" → 604800
    for part in cc.split(',') {
        let part = part.trim();
        if let Some(n) = part.strip_prefix("max-age=") {
            return n.trim().parse().ok();
        }
    }
    None
}
```

In `fetch_one`, pull the header before `.bytes()` is awaited (`reqwest`
gives us headers on `send()` completion):

```rust
let resp = self.http.get(&url).timeout(...).send().await;
let outcome = match resp {
    Ok(r) if r.status().is_success() => {
        let max_age = parse_max_age(r.headers())
            .map(Duration::from_secs)
            .unwrap_or(Duration::from_secs(default_max_age_secs()));
        match r.bytes().await {
            Ok(b) => IconState::Found {
                bytes: Arc::new(b.to_vec()),
                fetched_at: SystemTime::now(),
                max_age,
            },
            Err(_) => IconState::Missing {
                last_tried_at: SystemTime::now(),
                retry_after: Duration::from_secs(default_missing_retry_secs()),
            },
        }
    }
    _ => IconState::Missing {
        last_tried_at: SystemTime::now(),
        retry_after: Duration::from_secs(default_missing_retry_secs()),
    },
};
```

## Prefetch — stale-while-revalidate

`prefetch` decides per-hostname whether to schedule a fetch. The decision
matrix:

| Current state | Action |
|--|--|
| Not in cache | Mark `Pending`, fetch |
| `Pending` | Skip (fetch in flight) |
| `Found`, fresh | Skip |
| `Found`, stale | Fetch in background, keep old bytes rendered |
| `Missing`, retry-after not elapsed | Skip |
| `Missing`, retry-after elapsed | Mark `Pending`, fetch |

The "stale `Found`" case is the stale-while-revalidate path: the in-memory
entry stays as `Found` with the existing bytes so `get()` keeps returning
them. A parallel "refresh" task runs; when it completes, the entry is
swapped in place. If the refresh succeeds, the new PNG bytes replace the
old; if it fails, we update `fetched_at` to now + a short backoff so we
don't re-refresh on every ListLoaded. Concrete backoff: refresh failures
temporarily extend `max_age` by `min(original_max_age, 1h)` rather than
converting `Found` to `Missing`.

## When to refresh

- **Every ListLoaded**: scan the user's in-memory entries, collect stale
  ones, add them to the prefetch list alongside the genuinely-unknown
  hostnames.
- **No periodic timer**: we don't want to wake up every hour to refresh.
  ListLoaded is good enough for a password-manager — users open their vault
  infrequently, and when they do we refresh everything stale. A Phase 2.5
  could add a long-cadence `tokio::time::interval` if needed.

## Observability

- `tracing::info!` on batch completion: "refreshed N stale, fetched M new,
  skipped K fresh".
- Log `max-age` values at fetch time (debug) so we can sanity-check
  real-world server responses against the default.

## Verification checklist

- [ ] Unlock, let icons load. Inspect `index.json` — entries have
      `max_age_secs` matching the server's `Cache-Control` (or default if
      the header was absent).
- [ ] Manually edit `fetched_at` in `index.json` to a week ago, relaunch.
      Icons render immediately (stale bytes), then refresh in the
      background. `index.json` `fetched_at` updates.
- [ ] Unplug network before a stale refresh batch. Icons keep rendering;
      `fetched_at` doesn't move forward; retry backoff kicks in.
- [ ] Entry that was `Missing` yesterday with `retry_after_secs = 21600`
      retries today; entry missing 1 hour ago does not.
- [ ] `cargo clippy` clean.

## What this phase explicitly does not do

- No background timer-driven refresh (ListLoaded is the only trigger).
- No `ETag` / `If-None-Match` — we issue a full `GET` on refresh, not a
  conditional request. The Bitwarden icon service may or may not support
  conditional requests; test first before adding the complexity.
- No user-configurable TTL override. Defaults come from the server; if
  absent, 7 days.
- No eviction based on age. A stale entry stays on disk until the user
  logs out, even if the refresh permanently fails.

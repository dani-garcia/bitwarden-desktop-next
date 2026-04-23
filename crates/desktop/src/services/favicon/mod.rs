//! Favicon / cipher-icon service.
//!
//! Lazy, session-scoped: the first [`FaviconService::get`] call for a given
//! `(uid, hostname)` pair returns [`IconState::Pending`] and spawns a fetch;
//! subsequent calls are pure read-lock lookups. Fetch completions broadcast
//! a [`FaviconMessage::IconResolved`] that the App subscription turns into
//! a redraw. Nothing persists — matches the Angular clients (offline
//! launches show globes).
//!
//! `view()` only calls `get()` for virtualized rows currently on screen, so
//! "first sight of a hostname in the viewport" is what triggers a fetch.
//! Scroll naturally warms the next batch of icons.

use std::{
    collections::HashMap,
    sync::{Arc, OnceLock, RwLock},
};

use iced::{
    futures::{SinkExt, Stream, channel::mpsc},
    widget::image,
};
use tokio::sync::{Semaphore, broadcast};

use crate::domain::UserId;

pub type Hostname = String;

/// Rendered size for cipher-row icons (in logical pixels). Also the bitmap
/// dimensions we bake at fetch time so the rounded-corner alpha mask lines
/// up exactly with the rendered pixels.
pub const ICON_SIZE_PX: u32 = 32;

/// Corner radius used for the alpha mask. Matches `theme::RADIUS_SM` so the
/// bitmap edge and any surrounding container look identical.
const ICON_CORNER_RADIUS: f32 = 4.0;

/// Max concurrent HTTP fetches. Bounds the traffic a user's unlock or fast
/// scroll can generate against the icons server.
const FETCH_CONCURRENCY: usize = 5;

/// Per-request timeout. The icons server is usually fast; anything slower is
/// almost always broken.
const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(10);

// ── Public API ─────────────────────────────────────────────────────────────

/// Cheap-to-clone handle to the service. Clones share the same internal
/// state via `Arc`, so every field / view / handler that stores a copy sees
/// the same cache + fetch budget + broadcast.
#[derive(Clone)]
pub struct FaviconService {
    inner: Arc<FaviconInner>,
}

struct FaviconInner {
    http: reqwest::Client,
    caches: RwLock<HashMap<UserId, UserCache>>,
    fetch_budget: Arc<Semaphore>,
    resolve_icons_url: Arc<dyn Fn(&UserId) -> String + Send + Sync>,
    /// Runtime handle captured at construction. `get()` spawns from `view()`
    /// which is a synchronous context; using the handle explicitly avoids
    /// relying on tokio TLS being set on every iced render tick.
    runtime: tokio::runtime::Handle,
}

/// In-memory state for one user. Dropped whole on sign-out.
#[derive(Default)]
struct UserCache {
    entries: HashMap<Hostname, IconState>,
}

/// The render path only consumes [`Self::Found`] and falls back to the globe
/// for everything else (including no-URI, no-hostname, and Card/Identity/Note
/// cipher types). `Pending` exists to dedupe in-flight fetches; it renders
/// identically to `Missing`.
///
/// `Found` carries an already-constructed [`iced::widget::image::Handle`]
/// built from [`image::Handle::from_rgba`] at fetch time. Clones share the
/// internal `Id`, so iced's GPU texture cache hits on every re-render for
/// the session.
#[derive(Clone)]
pub enum IconState {
    Pending,
    Found(image::Handle),
    Missing,
}

/// Fetch-completion message delivered via [`favicon_event_stream`]. Arrival
/// drives the row's globe→favicon swap; the payload is only used for
/// tracing today.
#[derive(Debug, Clone)]
pub enum FaviconMessage {
    IconResolved { uid: UserId, hostname: Hostname },
}

impl FaviconService {
    pub fn new(resolve_icons_url: Arc<dyn Fn(&UserId) -> String + Send + Sync>) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .user_agent(concat!(
                "bitwarden-desktop-next/",
                env!("CARGO_PKG_VERSION")
            ))
            .use_rustls_tls()
            .build()
            .expect("reqwest client should build with rustls");

        Self {
            inner: Arc::new(FaviconInner {
                http,
                caches: RwLock::new(HashMap::new()),
                fetch_budget: Arc::new(Semaphore::new(FETCH_CONCURRENCY)),
                resolve_icons_url,
                runtime: tokio::runtime::Handle::current(),
            }),
        }
    }

    /// Render-time lookup. Fast path (already-known state) is a single read
    /// lock and a clone. Slow path (first sight) claims `Pending` under a
    /// write lock and spawns a background fetch; the fetch completion
    /// broadcasts an [`IconResolved`] that wakes the app subscription for
    /// a redraw.
    pub fn get(&self, uid: &UserId, hostname: &str) -> IconState {
        {
            let caches = self
                .inner
                .caches
                .read()
                .expect("favicon cache lock poisoned");
            if let Some(cache) = caches.get(uid)
                && let Some(state) = cache.entries.get(hostname)
            {
                return state.clone();
            }
        }
        self.claim_and_spawn(*uid, hostname.to_string());
        IconState::Pending
    }

    /// Drop the in-memory entry for `uid`. Called on log-out. In-flight
    /// fetches for this user stay alive until they finish but their results
    /// land in `fetch_and_commit` which no-ops when the cache entry is gone.
    pub fn evict_user(&self, uid: &UserId) {
        self.inner
            .caches
            .write()
            .expect("favicon cache lock poisoned")
            .remove(uid);
    }

    fn claim_and_spawn(&self, uid: UserId, hostname: Hostname) {
        // Double-check under write lock: a concurrent `get()` for the same
        // row across a quick scroll could have already inserted Pending.
        {
            let mut caches = self
                .inner
                .caches
                .write()
                .expect("favicon cache lock poisoned");
            let cache = caches.entry(uid).or_default();
            if cache.entries.contains_key(&hostname) {
                return;
            }
            cache.entries.insert(hostname.clone(), IconState::Pending);
        }
        let inner = Arc::clone(&self.inner);
        self.inner.runtime.spawn(async move {
            fetch_and_commit(inner, uid, hostname).await;
        });
    }
}

// ── Hostname extraction ────────────────────────────────────────────────────

/// Extract the hostname we'd use to fetch a favicon from a raw cipher URI,
/// or return `None` if the URI is unsuitable (non-http(s) scheme, bare IP,
/// single-label, tor / i2p).
///
/// Matches the Angular clients' `Utils.getHostname()` (`tldts`' `getHostname`),
/// which returns the full hostname — not eTLD+1. `mail.google.com` and
/// `google.com` produce different cache entries.
pub fn hostname_for_fetch(uri: &str) -> Option<Hostname> {
    let parsed = url::Url::parse(uri)
        .or_else(|_| url::Url::parse(&format!("http://{uri}")))
        .ok()?;
    match parsed.scheme() {
        "http" | "https" => {}
        _ => return None,
    }
    let host = parsed.host_str()?.to_ascii_lowercase();
    if host.parse::<std::net::IpAddr>().is_ok() {
        return None;
    }
    if !host.contains('.') {
        return None;
    }
    if host.ends_with(".onion") || host.ends_with(".i2p") {
        return None;
    }
    Some(host)
}

// ── Broadcast fan-out (iced subscription) ─────────────────────────────────

/// Process-global fan-out for fetch completions. Lazily initialized on first
/// send or subscribe — matches the [`crate::services::menu`] muda pattern. The
/// channel's initial receiver is dropped; [`broadcast::Sender::subscribe`]
/// still works for subsequent subscribers, and `send()` silently no-ops
/// when there are zero active receivers (which we ignore anyway).
static EVENTS: OnceLock<broadcast::Sender<FaviconMessage>> = OnceLock::new();

fn events() -> &'static broadcast::Sender<FaviconMessage> {
    EVENTS.get_or_init(|| broadcast::channel::<FaviconMessage>(1024).0)
}

/// Iced-compatible stream of fetch completions. Use as a `fn` pointer with
/// [`iced::Subscription::run`]; iced hashes the function identity so the
/// same stream instance persists across `update()` cycles instead of
/// rebinding each tick.
pub fn favicon_event_stream() -> impl Stream<Item = FaviconMessage> {
    iced::stream::channel(32, |mut out: mpsc::Sender<FaviconMessage>| async move {
        let mut rx = events().subscribe();
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    if out.send(ev).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(dropped = n, "favicon event subscriber lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    })
}

// ── Fetch pipeline ─────────────────────────────────────────────────────────

async fn fetch_and_commit(inner: Arc<FaviconInner>, uid: UserId, hostname: Hostname) {
    let state = fetch_one(&inner, &uid, &hostname).await;

    let committed = {
        let mut caches = inner.caches.write().expect("favicon cache lock poisoned");
        if let Some(cache) = caches.get_mut(&uid) {
            cache.entries.insert(hostname.clone(), state);
            true
        } else {
            // User was evicted mid-fetch; drop the result silently.
            false
        }
    };
    if committed {
        let _ = events().send(FaviconMessage::IconResolved { uid, hostname });
    }
}

async fn fetch_one(inner: &FaviconInner, uid: &UserId, hostname: &Hostname) -> IconState {
    // Dropping the permit on return is what enforces the 5-concurrent cap.
    let Ok(_permit) = inner.fetch_budget.clone().acquire_owned().await else {
        // Semaphore closed — the service is shutting down.
        return IconState::Missing;
    };

    let url = format!(
        "{}/{}/icon.png",
        (inner.resolve_icons_url)(uid).trim_end_matches('/'),
        hostname
    );
    let resp = inner.http.get(&url).timeout(FETCH_TIMEOUT).send().await;
    match resp {
        Ok(r) if r.status().is_success() => match r.bytes().await {
            Ok(body) => decode_to_handle(body.to_vec()).await.unwrap_or_else(|e| {
                tracing::debug!(host = %hostname, error = %e, "favicon decode failed");
                IconState::Missing
            }),
            Err(e) => {
                tracing::debug!(host = %hostname, error = %e, "favicon body read failed");
                IconState::Missing
            }
        },
        Ok(r) => {
            tracing::debug!(host = %hostname, status = %r.status(), "favicon non-success");
            IconState::Missing
        }
        Err(e) => {
            tracing::debug!(host = %hostname, error = %e, "favicon request failed");
            IconState::Missing
        }
    }
}

// ── Decode + alpha-mask pipeline ──────────────────────────────────────────

/// Decode raw bytes on a blocking worker, apply the rounded-rect mask, and
/// wrap the result in a cached-Id iced handle. The `image` crate's decode is
/// pure CPU; with up to 5 landing at once we keep the tokio runtime's worker
/// threads free.
async fn decode_to_handle(raw: Vec<u8>) -> Result<IconState, DecodeError> {
    let rgba = tokio::task::spawn_blocking(move || decode_and_mask(&raw))
        .await
        .map_err(|e| DecodeError(e.to_string()))?
        .map_err(|e| DecodeError(e.to_string()))?;
    let (w, h) = (rgba.width(), rgba.height());
    Ok(IconState::Found(image::Handle::from_rgba(
        w,
        h,
        rgba.into_raw(),
    )))
}

#[derive(Debug)]
struct DecodeError(String);

impl std::fmt::Display for DecodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// Decode raw bytes, resize to the icon slot, and punch the rounded-rect
/// alpha mask. The returned `RgbaImage` is ready to be fed into
/// [`image::Handle::from_rgba`].
fn decode_and_mask(raw: &[u8]) -> Result<::image::RgbaImage, ::image::ImageError> {
    let img = ::image::load_from_memory(raw)?;
    // The icons service returns PNGs sized to the requested scale; we render
    // at 32 px regardless, so a fixed resize keeps the alpha mask exact.
    let resized = img.resize_exact(
        ICON_SIZE_PX,
        ICON_SIZE_PX,
        ::image::imageops::FilterType::Lanczos3,
    );
    let mut rgba = resized.to_rgba8();
    apply_rounded_mask(&mut rgba, ICON_CORNER_RADIUS);
    Ok(rgba)
}

/// Multiply every pixel's alpha by a rounded-rect coverage value. One-pixel
/// wide band around the corner edge is antialiased so the result doesn't
/// shimmer at our 32px size.
fn apply_rounded_mask(img: &mut ::image::RgbaImage, radius: f32) {
    let w = img.width() as f32;
    let h = img.height() as f32;
    if radius <= 0.0 {
        return;
    }
    let r = radius.min(w / 2.0).min(h / 2.0);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        let px = x as f32 + 0.5;
        let py = y as f32 + 0.5;
        // Closest point on the inner rectangle whose corner arcs have centre
        // (r, r), (w-r, r), (r, h-r), (w-r, h-r). When the pixel falls in the
        // straight-edge region the clamped point equals the pixel itself and
        // `dist` is 0 → fully covered.
        let cx = px.clamp(r, w - r);
        let cy = py.clamp(r, h - r);
        let dx = px - cx;
        let dy = py - cy;
        let dist = (dx * dx + dy * dy).sqrt();
        // Coverage: 1 fully inside, 0 fully outside, linear across a 1-pixel
        // band. Matches iced's own container antialiasing closely enough
        // that a Found icon sits on any theme background without a seam.
        let coverage = (r + 0.5 - dist).clamp(0.0, 1.0);
        if coverage < 1.0 {
            let a = pixel.0[3] as f32;
            pixel.0[3] = (a * coverage) as u8;
        }
    }
}

// ── Globe fallback handle ──────────────────────────────────────────────────

pub fn globe_handle() -> image::Handle {
    image::Handle::from_bytes(crate::assets::BWI_GLOBE_PNG)
}

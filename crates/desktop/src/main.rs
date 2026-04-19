#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod clipboard;
mod components;
mod i18n;
mod menu;
mod sdk;
mod state;
mod theme;
mod views;

/// Convenience wrapper around [`i18n_embed_fl::fl!`] that passes our static
/// [`i18n::LANGUAGE_LOADER`] implicitly. Usage:
///
/// ```ignore
/// fl!("login-unlock-title")
/// fl!("login-server-accessing", server = "bitwarden.com")
/// ```
///
/// Unknown message IDs or wrong argument names are a **compile error** — the
/// macro validates against the `.ftl` files under `i18n/` at build time.
#[macro_export]
macro_rules! fl {
    ($message_id:literal) => {{
        ::i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id)
    }};
    ($message_id:literal, $($args:expr),*) => {{
        ::i18n_embed_fl::fl!($crate::i18n::LANGUAGE_LOADER, $message_id, $($args),*)
    }};
}

use iced::{
    Font,
    font::{Family, Weight},
};

use crate::{app::App, components::icons};

/// App-wide default font (Inter 18pt Medium).
pub const APP_FONT: Font = Font {
    family: Family::Name("Inter 18pt"),
    weight: Weight::Medium,
    stretch: iced::font::Stretch::Normal,
    style: iced::font::Style::Normal,
};

/// Bold variant of the app font.
pub const APP_FONT_BOLD: Font = Font {
    weight: Weight::Bold,
    ..APP_FONT
};

fn main() -> iced::Result {
    init_tracing();
    i18n::init();

    // Default to tiny-skia (CPU) renderer to avoid ~500 ms wgpu GPU init on
    // startup. For a form-based UI this is fast enough, and the instant
    // window appearance is a better UX trade-off. The `gpu` Cargo feature
    // enables the wgpu backend; `--gpu` then opts into it at runtime.
    // Without the feature, the flag is a no-op.
    let backend = if cfg!(feature = "gpu") && std::env::args().any(|a| a == "--gpu") {
        "wgpu"
    } else {
        "tiny-skia"
    };

    tracing::info!("Starting app with Iced backend: {}", backend);
    // SAFETY: called at the very start of main, before any threads are spawned.
    unsafe { std::env::set_var("ICED_BACKEND", backend) };

    iced::daemon(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title(App::title)
        .theme(App::theme)
        .font(assets::FONT_MEDIUM)
        .font(assets::FONT_BOLD)
        .font(assets::BWI_FONT)
        .font(icons::FONT_BYTES)
        .default_font(APP_FONT)
        .antialiasing(true)
        .run()
}

/// Install a `tracing` subscriber driven by the `RUST_LOG` env var. The default
/// filter (`bitwarden_desktop_next=debug,warn`) gives our crate verbose output
/// while keeping dependency noise quiet. The SDK crates already emit
/// `#[tracing::instrument]` spans (unlock, crypto init, vault decrypt) so once
/// this runs they become visible for free under e.g.
/// `RUST_LOG=bitwarden_desktop_next=debug,bitwarden_core=debug cargo run`.
fn init_tracing() {
    use tracing_subscriber::{EnvFilter, fmt};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("bitwarden_desktop_next=debug,warn"));

    fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_writer(std::io::stderr)
        .init();
}

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod components;
mod menu;
mod sdk;
mod state;
mod theme;
mod views;

use iced::{
    Font, Size,
    font::{Family, Weight},
    window::{self, settings::PlatformSpecific},
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

    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("Bitwarden [Next]")
        .font(assets::FONT_MEDIUM)
        .font(assets::FONT_BOLD)
        .font(assets::BWI_FONT)
        .font(icons::FONT_BYTES)
        .default_font(APP_FONT)
        .window(window::Settings {
            size: Size::new(1024.0, 800.0),
            min_size: Some(Size::new(800.0, 750.0)),
            decorations: menu::should_use_native_title_bar(),
            platform_specific: get_platform_specific(),
            icon: window::icon::from_file_data(assets::ICON_PNG, Some(image::ImageFormat::Png))
                .ok(),
            ..Default::default()
        })
        .theme(|app: &App| app.theme.current.clone())
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
        .with_target(false)
        .with_writer(std::io::stderr)
        .init();
}

fn get_platform_specific() -> PlatformSpecific {
    #[cfg(target_os = "windows")]
    {
        PlatformSpecific {
            undecorated_shadow: true,
            corner_preference: window::settings::platform::CornerPreference::Round,
            ..Default::default()
        }
    }

    #[cfg(target_os = "macos")]
    {
        PlatformSpecific {
            title_hidden: true,
            titlebar_transparent: true,
            fullsize_content_view: true,
        }
    }

    #[cfg(not(any(target_os = "windows", target_os = "macos")))]
    {
        PlatformSpecific::default()
    }
}

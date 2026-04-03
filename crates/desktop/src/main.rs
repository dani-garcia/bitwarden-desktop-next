#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod assets;
mod components;
mod menu;
mod mock;
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
        .theme(|app: &App| app.current_theme.clone())
        .antialiasing(true)
        .run()
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

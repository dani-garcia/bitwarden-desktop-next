#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
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

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("Bitwarden [Next]")
        .font(include_bytes!("../assets/InterVariable.ttf"))
        .font(icons::FONT_BYTES)
        .font(icons::BWI_FONT_BYTES)
        .default_font(Font {
            family: Family::Name("Inter"),
            weight: Weight::Normal,
            ..Font::DEFAULT
        })
        .window(window::Settings {
            size: Size::new(1024.0, 800.0),
            min_size: Some(Size::new(800.0, 750.0)),
            decorations: menu::should_use_native_title_bar(),
            platform_specific: get_platform_specific(),
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

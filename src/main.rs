#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
pub mod components;
mod menu;
mod mock;
mod state;
pub mod theme;
mod views;

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("Bitwarden [Next]")
        .font(include_bytes!("../assets/InterVariable.ttf").as_slice())
        .font(components::icons::FONT_BYTES)
        .font(components::icons::BWI_FONT_BYTES)
        .default_font(iced::Font {
            family: iced::font::Family::Name("Inter"),
            weight: iced::font::Weight::Normal,
            ..iced::Font::DEFAULT
        })
        .window(iced::window::Settings {
            size: iced::Size::new(1024.0, 800.0),
            min_size: Some(iced::Size::new(800.0, 750.0)),
            decorations: cfg!(target_os = "macos"),
            platform_specific: iced::window::settings::platform::PlatformSpecific {
                undecorated_shadow: true,
                ..Default::default()
            },
            ..Default::default()
        })
        .theme(|app: &App| app.current_theme.clone())
        .antialiasing(true)
        .run()
}

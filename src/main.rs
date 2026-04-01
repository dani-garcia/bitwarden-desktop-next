#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod menu;
mod mock;
mod state;
mod theme;
mod views;
mod widgets;

use app::App;

fn main() -> iced::Result {
    iced::application(App::new, App::update, App::view)
        .subscription(App::subscription)
        .title("Bitwarden [Next]")
        .font(include_bytes!("../assets/InterVariable.ttf").as_slice())
        .default_font(iced::Font {
            family: iced::font::Family::Name("Inter"),
            weight: iced::font::Weight::Normal,
            ..iced::Font::DEFAULT
        })
        .window(iced::window::Settings {
            size: iced::Size::new(1080.0, 720.0),
            min_size: Some(iced::Size::new(600.0, 480.0)),
            ..Default::default()
        })
        .run()
}

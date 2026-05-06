mod handler;

use iced::{
    Alignment, Element, Fill, Length,
    widget::{Space, column, container, row, text},
};

use crate::{app::RenderCtx, components::buttons, fl, theme::AppTheme};

#[derive(Debug, Clone)]
pub enum AboutMessage {
    CopyInfo,
    Close,
}

/// Pinned SDK revision, displayed in the About dialog. Keep in sync with
/// `bitwarden-*` git revs in the workspace root `Cargo.toml`.
/// TODO: automate via a build.rs that parses the workspace manifest.
const SDK_REV_SHORT: &str = "6c25425";

/// Build the multi-line info string for the clipboard Copy action.
pub fn info_string() -> String {
    format!(
        "Bitwarden\nVersion {}\nSDK version {}\nOS {}\nArchitecture {}",
        env!("CARGO_PKG_VERSION"),
        SDK_REV_SHORT,
        std::env::consts::OS,
        std::env::consts::ARCH,
    )
}

pub(crate) fn view<'a>(ctx: &RenderCtx<'a>) -> Element<'a, AboutMessage, AppTheme> {
    let colors = ctx.colors;
    // "Bitwarden" is the product name — kept untranslated.
    let title = text("Bitwarden")
        .size(28)
        .color(colors.text_primary)
        .font(crate::APP_FONT_BOLD);

    let info_line = |label: String, value: String| -> Element<'_, AboutMessage, AppTheme> {
        row![
            text(label).size(14).color(colors.text_secondary),
            Space::new().width(Fill),
            text(value).size(14).color(colors.text_primary),
        ]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
    };

    let info = column![
        info_line(
            fl!("about-version-label"),
            env!("CARGO_PKG_VERSION").to_string()
        ),
        info_line(fl!("about-sdk-version-label"), SDK_REV_SHORT.to_string()),
        info_line(fl!("about-os-label"), std::env::consts::OS.to_string()),
        info_line(
            fl!("about-architecture-label"),
            std::env::consts::ARCH.to_string()
        ),
    ]
    .spacing(6);

    let copy_button = buttons::secondary(text(fl!("about-copy-button")).size(14))
        .on_press(AboutMessage::CopyInfo)
        .padding([8, 16]);
    let close_button = buttons::primary(text(fl!("about-close-button")).size(14))
        .on_press(AboutMessage::Close)
        .padding([8, 16]);

    let buttons = row![Space::new().width(Fill), copy_button, close_button]
        .spacing(8)
        .align_y(Alignment::Center);

    container(
        column![
            title,
            info,
            Space::new().height(Length::Fixed(8.0)),
            buttons
        ]
        .spacing(16)
        .width(Fill),
    )
    .padding(24)
    .width(Fill)
    .height(Fill)
    .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background))
    .into()
}

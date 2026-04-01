use iced::widget::{button, column, container, row, scrollable, text, Column};
use iced::{Alignment, Element, Fill, Padding};

use crate::state::CipherItem;
use crate::theme;

#[derive(Debug, Clone)]
pub enum ItemListMessage {
    ItemSelected(usize),
}

// Generate a deterministic color from a string
fn initial_color(name: &str) -> iced::Color {
    let hash: u32 = name.bytes().fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (hash % 360) as f32;
    // HSL to RGB with fixed saturation=0.5, lightness=0.45
    let s: f32 = 0.5;
    let l: f32 = 0.45;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match hue as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    iced::Color::from_rgb(r + m, g + m, b + m)
}

pub fn view<'a>(
    items: &'a [CipherItem],
    selected_index: Option<usize>,
) -> Element<'a, ItemListMessage> {
    let item_rows: Vec<Element<'a, ItemListMessage>> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_index == Some(i);
            let subtitle = item
                .username
                .as_deref()
                .or(item.url.as_deref())
                .unwrap_or("");

            let initial = item
                .name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            let circle_color = initial_color(&item.name);

            let icon_circle = container(
                text(initial)
                    .size(13)
                    .color(theme::TEXT_PRIMARY),
            )
            .width(32)
            .height(32)
            .center_x(32)
            .center_y(32)
            .style(move |_theme| container::Style {
                background: Some(iced::Background::Color(circle_color)),
                border: iced::Border {
                    radius: 16.0.into(),
                    ..Default::default()
                },
                ..Default::default()
            });

            let info = column![
                text(&item.name).size(14).color(theme::TEXT_PRIMARY),
                text(subtitle).size(12).color(theme::TEXT_SECONDARY),
            ]
            .spacing(2);

            let content = row![icon_circle, info]
                .spacing(12)
                .align_y(Alignment::Center);

            let bg = if is_selected {
                theme::SELECTED_BG
            } else {
                iced::Color::TRANSPARENT
            };

            button(container(content).padding(Padding::from([4.0, 0.0])))
                .on_press(ItemListMessage::ItemSelected(i))
                .padding(Padding::from([4.0, 16.0]))
                .width(Fill)
                .style(move |_theme, status| {
                    let bg_color = match status {
                        button::Status::Hovered if !is_selected => theme::ITEM_HOVER,
                        _ => bg,
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg_color)),
                        text_color: theme::TEXT_PRIMARY,
                        border: iced::Border::default(),
                        shadow: iced::Shadow::default(),
                        snap: false,
                    }
                })
                .into()
        })
        .collect();

    let list = Column::with_children(item_rows).spacing(1);

    container(scrollable(list).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BACKGROUND)),
            ..Default::default()
        })
        .into()
}

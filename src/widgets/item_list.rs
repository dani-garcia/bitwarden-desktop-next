use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow,
    widget::{Column, Space, button, column, container, row, scrollable, text},
};

use crate::{icons, state::CipherItem, theme, widgets::common};

#[derive(Debug, Clone)]
#[allow(dead_code)] // usize fields used at emit site, not yet read by handler
pub enum ItemListMessage {
    ItemSelected(usize),
    OpenExternal(usize),
    CopyUsername(usize),
    MoreOptions(usize),
}

// Generate a deterministic color from a string
fn initial_color(name: &str) -> iced::Color {
    let hash: u32 = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (hash % 360) as f32;
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
    Color::from_rgb(r + m, g + m, b + m)
}

pub fn view<'a>(
    items: &'a [CipherItem],
    selected_index: Option<usize>,
) -> Element<'a, ItemListMessage> {
    // Table header
    let table_header = container(
        row![
            text("Name").size(13).color(theme::TABLE_HEADER),
            icons::ARROW_DOWN_UP.render(11.0, theme::TABLE_HEADER),
            Space::new().width(Fill),
            text("Options").size(13).color(theme::TABLE_HEADER),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([8, 24])
    .width(Fill);

    let header_divider = container(common::separator_h()).padding([0, 16]);

    // Item rows
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

            let icon_circle = container(text(initial).size(13).color(theme::TEXT_PRIMARY))
                .width(32)
                .height(32)
                .center_x(32)
                .center_y(32)
                .style(move |_theme| container::Style {
                    background: Some(Background::Color(circle_color)),
                    border: Border {
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

            // Action icons
            let mut actions: Vec<Element<'a, ItemListMessage>> = Vec::new();
            if item.url.is_some() {
                actions.push(action_icon(
                    icons::BOX_ARROW_UP_RIGHT,
                    ItemListMessage::OpenExternal(i),
                ));
            }
            if item.username.is_some() {
                actions.push(action_icon(icons::COPY, ItemListMessage::CopyUsername(i)));
            }
            actions.push(action_icon(
                icons::THREE_DOTS_VERTICAL,
                ItemListMessage::MoreOptions(i),
            ));

            let actions_row = row(actions).spacing(2).align_y(Alignment::Center);

            let content = row![icon_circle, info, Space::new().width(Fill), actions_row]
                .spacing(12)
                .align_y(Alignment::Center);

            container(
                button(content)
                    .on_press(ItemListMessage::ItemSelected(i))
                    .padding([8, 8])
                    .width(Fill)
                    .style(move |_theme, status| {
                        common::hover_button_style(
                            status,
                            is_selected,
                            theme::ITEM_HOVER,
                            theme::RADIUS_MD,
                        )
                    }),
            )
            .padding([0, 16])
            .into()
        })
        .collect();

    let mut list_items: Vec<Element<'a, ItemListMessage>> = Vec::new();
    for (i, row) in item_rows.into_iter().enumerate() {
        if i > 0 {
            list_items.push(container(common::separator_h()).padding([0, 16]).into());
        }
        list_items.push(row);
    }

    let list = Column::with_children(list_items);

    let styled_scrollable =
        scrollable(list)
            .height(Fill)
            .style(|_theme, _status| scrollable::Style {
                container: container::Style::default(),
                vertical_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: Background::Color(theme::ITEM_HOVER),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                    },
                },
                horizontal_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: Background::Color(theme::ITEM_HOVER),
                        border: Border {
                            radius: 4.0.into(),
                            ..Default::default()
                        },
                    },
                },
                gap: None,
                auto_scroll: scrollable::AutoScroll {
                    background: Background::Color(Color::TRANSPARENT),
                    border: Border::default(),
                    shadow: Shadow::default(),
                    icon: Color::TRANSPARENT,
                },
            });

    column![table_header, header_divider, styled_scrollable]
        .width(Fill)
        .height(Fill)
        .into()
}

fn action_icon<'a>(icon: icons::Icon, message: ItemListMessage) -> Element<'a, ItemListMessage> {
    button(icon.render(14.0, theme::TEXT_SECONDARY))
        .on_press(message)
        .padding([4, 6])
        .style(|_theme, status| {
            common::hover_button_style(status, false, Color::TRANSPARENT, theme::RADIUS_SM)
        })
        .into()
}

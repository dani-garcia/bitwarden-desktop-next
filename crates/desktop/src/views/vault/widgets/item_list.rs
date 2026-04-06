use std::sync::Arc;

use bitwarden_vault::{CipherListView, CipherListViewType};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow,
    widget::{Column, Space, column, container, row, scrollable, text},
};

use crate::{components::{self, buttons, icons}, theme::{AppColors, AppTheme, RADIUS_MD}};

#[derive(Debug, Clone)]
pub enum ItemListMessage {
    ItemSelected(usize),
    OpenExternal(#[expect(dead_code)] usize),
    CopyUsername(#[expect(dead_code)] usize),
    MoreOptions(#[expect(dead_code)] usize),
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
    items: &'a [Arc<CipherListView>],
    selected_index: Option<usize>,
    colors: &AppColors,
) -> Element<'a, ItemListMessage, AppTheme> {
    // Table header
    let table_header = container(
        row![
            text("Name").size(14).color(colors.table_header).font(crate::APP_FONT_BOLD),
            icons::ARROW_DOWN_UP.render(11.0, colors.table_header),
            Space::new().width(Fill),
            text("Options").size(14).color(colors.table_header).font(crate::APP_FONT_BOLD),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([8, 24])
    .width(Fill);

    let header_divider = container(components::separator_h()).padding([0, 16]);

    // Item rows
    let item_rows: Vec<Element<'a, ItemListMessage, AppTheme>> = items
        .iter()
        .enumerate()
        .map(|(i, item)| {
            let is_selected = selected_index == Some(i);
            let subtitle = item.subtitle.as_str();
            let (has_username, has_uri) = match &item.r#type {
                CipherListViewType::Login(login) => (
                    login.username.is_some(),
                    login
                        .uris
                        .as_ref()
                        .and_then(|u| u.first())
                        .and_then(|u| u.uri.as_deref())
                        .is_some(),
                ),
                _ => (false, false),
            };

            let initial = item
                .name
                .chars()
                .next()
                .unwrap_or('?')
                .to_uppercase()
                .to_string();
            let circle_color = initial_color(&item.name);

            let icon_circle = container(text(initial).size(14).color(colors.text_primary))
                .width(32)
                .height(32)
                .center_x(32)
                .center_y(32)
                .style(move |_theme: &AppTheme| container::Style {
                    background: Some(Background::Color(circle_color)),
                    border: Border {
                        radius: 16.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                });

            let info = column![
                text(&item.name).size(14).color(colors.text_primary),
                text(subtitle).size(14).color(colors.text_secondary),
            ]
            .spacing(2);

            // Action icons
            let mut actions: Vec<Element<'a, ItemListMessage, AppTheme>> = Vec::new();
            if has_uri {
                actions.push(action_icon(
                    icons::BOX_ARROW_UP_RIGHT,
                    ItemListMessage::OpenExternal(i),
                    colors,
                ));
            }
            if has_username {
                actions.push(action_icon(icons::COPY, ItemListMessage::CopyUsername(i), colors));
            }
            actions.push(action_icon(
                icons::THREE_DOTS_VERTICAL,
                ItemListMessage::MoreOptions(i),
                colors,
            ));

            let actions_row = row(actions).spacing(2).align_y(Alignment::Center);

            let content = row![icon_circle, info, Space::new().width(Fill), actions_row]
                .spacing(12)
                .align_y(Alignment::Center);

            container(
                buttons::ghost(content, is_selected, colors.item_hover, colors.item_hover, RADIUS_MD)
                    .on_press(ItemListMessage::ItemSelected(i))
                    .padding([8, 8])
                    .width(Fill),
            )
            .padding([0, 16])
            .into()
        })
        .collect();

    let mut list_items: Vec<Element<'a, ItemListMessage, AppTheme>> = Vec::new();
    for (i, row) in item_rows.into_iter().enumerate() {
        if i > 0 {
            list_items.push(container(components::separator_h()).padding([0, 16]).into());
        }
        list_items.push(row);
    }

    let list = Column::with_children(list_items);

    let styled_scrollable =
        scrollable(list)
            .height(Fill)
            .style(|theme: &AppTheme, _status| scrollable::Style {
                container: container::Style::default(),
                vertical_rail: scrollable::Rail {
                    background: None,
                    border: Border::default(),
                    scroller: scrollable::Scroller {
                        background: Background::Color(theme.colors.item_hover),
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
                        background: Background::Color(theme.colors.item_hover),
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

fn action_icon<'a>(icon: icons::Icon, message: ItemListMessage, colors: &AppColors) -> Element<'a, ItemListMessage, AppTheme> {
    buttons::ghost_icon(icon.render(14.0, colors.text_secondary), colors.item_hover)
        .on_press(message)
        .padding([4, 6])
        .into()
}

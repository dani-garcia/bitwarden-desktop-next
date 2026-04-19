use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow,
    widget::{button, column, container, mouse_area, row, text},
};

use crate::{
    components,
    menu::{MenuEntry, MenuState},
    theme::{AppColors, AppTheme},
};

use super::{DROPDOWN_WIDTH, ITEM_PADDING, PANEL_RADIUS, SUBMENU_WIDTH, TitleBarMessage};

/// Build the overlay content for a single menu's dropdown.
/// When a submenu is open, returns a row with the main panel and submenu side-by-side.
pub fn menu_panel<'a>(
    entries: &[MenuEntry],
    menu_index: usize,
    open_submenu: Option<usize>,
    state: &MenuState,
    colors: &AppColors,
) -> Element<'a, TitleBarMessage, AppTheme> {
    let items = render_entries(entries, menu_index, open_submenu, state, colors);
    let main_panel = styled_panel(column(items).spacing(2).width(DROPDOWN_WIDTH));

    if let Some(sub_idx) = open_submenu
        && let Some(entry) = entries.get(sub_idx)
        && !entry.children.is_empty()
    {
        let v_offset: f32 = entries[..sub_idx]
            .iter()
            .map(|e| {
                if e.is_separator() {
                    9.0
                } else {
                    ITEM_PADDING.top + ITEM_PADDING.bottom + 16.0
                }
            })
            .sum::<f32>()
            + 6.0;

        let sub_items: Vec<Element<'_, TitleBarMessage, AppTheme>> = entry
            .children
            .iter()
            .enumerate()
            .map(|(sub_i, sub_entry)| {
                render_submenu_item(sub_entry, menu_index, sub_idx, sub_i, state, colors)
            })
            .collect();

        let sub_panel = styled_panel(column(sub_items).spacing(2).width(SUBMENU_WIDTH));
        let sub_with_offset = column![iced::widget::Space::new().height(v_offset), sub_panel];

        return row![main_panel, sub_with_offset].spacing(4).into();
    }

    main_panel
}

fn styled_panel<'a>(
    content: impl Into<Element<'a, TitleBarMessage, AppTheme>>,
) -> Element<'a, TitleBarMessage, AppTheme> {
    container(content)
        .padding([6, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(PANEL_RADIUS),
                )
        })
        .into()
}

fn render_entries<'a>(
    entries: &[MenuEntry],
    menu_index: usize,
    open_submenu: Option<usize>,
    state: &MenuState,
    colors: &AppColors,
) -> Vec<Element<'a, TitleBarMessage, AppTheme>> {
    entries
        .iter()
        .enumerate()
        .map(|(item_i, entry)| render_entry(entry, menu_index, item_i, open_submenu, state, colors))
        .collect()
}

fn render_entry<'a>(
    entry: &MenuEntry,
    menu_index: usize,
    item_index: usize,
    open_submenu: Option<usize>,
    state: &MenuState,
    colors: &AppColors,
) -> Element<'a, TitleBarMessage, AppTheme> {
    if entry.is_separator() {
        return container(components::separator_h()).padding([2, 8]).into();
    }

    let enabled = entry.is_enabled(state);
    let is_sub = entry.is_submenu();
    let sub_is_open = is_sub && open_submenu == Some(item_index);

    let label_color = if enabled {
        colors.text_primary
    } else {
        colors.text_muted
    };
    let shortcut_color = if enabled {
        colors.text_secondary
    } else {
        colors.text_muted
    };

    let label_text = text(entry.display_label()).size(14).color(label_color);

    let content: Element<'_, TitleBarMessage, AppTheme> = if is_sub {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            crate::components::icons::CHEVRON_RIGHT.render(11.0, shortcut_color),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    } else if let Some(shortcut_text) = entry.shortcut_display() {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            text(shortcut_text).size(12).color(shortcut_color),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    } else {
        label_text.into()
    };

    let mut btn =
        button(content)
            .padding(ITEM_PADDING)
            .width(Fill)
            .style(move |theme: &AppTheme, status| {
                let bg = if sub_is_open {
                    theme.colors.item_hover
                } else if enabled {
                    match status {
                        button::Status::Hovered => theme.colors.item_hover,
                        _ => Color::TRANSPARENT,
                    }
                } else {
                    Color::TRANSPARENT
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: label_color,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: false,
                }
            });

    if enabled {
        btn = btn.on_press(TitleBarMessage::ItemClicked(menu_index, item_index));
    }

    if is_sub && enabled {
        mouse_area(btn)
            .on_enter(TitleBarMessage::SubMenuHovered(menu_index, item_index))
            .into()
    } else if enabled {
        mouse_area(btn)
            .on_enter(TitleBarMessage::SubMenuHovered(menu_index, usize::MAX))
            .into()
    } else {
        btn.into()
    }
}

fn render_submenu_item<'a>(
    entry: &MenuEntry,
    menu_index: usize,
    parent_index: usize,
    sub_index: usize,
    state: &MenuState,
    colors: &AppColors,
) -> Element<'a, TitleBarMessage, AppTheme> {
    if entry.is_separator() {
        return container(components::separator_h()).padding([2, 8]).into();
    }

    let enabled = entry.is_enabled(state);
    let label_color = if enabled {
        colors.text_primary
    } else {
        colors.text_muted
    };

    let label_text = text(entry.display_label()).size(14).color(label_color);

    let content: Element<'_, TitleBarMessage, AppTheme> =
        if let Some(shortcut_text) = entry.shortcut_display() {
            row![
                label_text,
                iced::widget::Space::new().width(Fill),
                text(shortcut_text).size(12).color(if enabled {
                    colors.text_secondary
                } else {
                    colors.text_muted
                }),
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        } else {
            label_text.into()
        };

    let mut btn =
        button(content)
            .padding(ITEM_PADDING)
            .width(Fill)
            .style(move |theme: &AppTheme, status| {
                let bg = if enabled {
                    match status {
                        button::Status::Hovered => theme.colors.item_hover,
                        _ => Color::TRANSPARENT,
                    }
                } else {
                    Color::TRANSPARENT
                };
                button::Style {
                    background: Some(Background::Color(bg)),
                    text_color: label_color,
                    border: Border::default(),
                    shadow: Shadow::default(),
                    snap: false,
                }
            });

    if enabled {
        btn = btn.on_press(TitleBarMessage::SubMenuItemClicked(
            menu_index,
            parent_index,
            sub_index,
        ));
    }

    btn.into()
}

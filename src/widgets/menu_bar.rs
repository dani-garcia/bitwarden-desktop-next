use iced::widget::{button, column, container, mouse_area, row, rule, text};
use iced::{Alignment, Element, Fill, Padding};

use crate::menu::{self, MenuEntry, MenuState};
use crate::theme;

const DROPDOWN_WIDTH: f32 = 280.0;
const SUBMENU_WIDTH: f32 = 220.0;
const ITEM_PADDING: Padding = Padding { top: 8.0, right: 16.0, bottom: 8.0, left: 16.0 };
const PANEL_RADIUS: f32 = 8.0;

#[derive(Debug, Clone)]
pub enum MenuBarMessage {
    TopLevelClicked(usize),
    TopLevelHovered(usize),
    ItemClicked(usize, usize),
    SubMenuHovered(usize, usize),
    SubMenuItemClicked(usize, usize, usize),
}

/// Draw the menu bar row (top-level labels).
pub fn view<'a>(open_menu: Option<usize>) -> Element<'a, MenuBarMessage> {
    let items: Vec<Element<'_, MenuBarMessage>> = menu::MENUS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let is_open = open_menu == Some(i);
            let btn = button(text(*label).size(13).color(theme::TEXT_PRIMARY))
                .on_press(MenuBarMessage::TopLevelClicked(i))
                .padding(Padding::from([4.0, 10.0]))
                .style(move |_theme, status| {
                    let bg = if is_open {
                        theme::CARD_BG
                    } else {
                        match status {
                            button::Status::Hovered => theme::ITEM_HOVER,
                            _ => iced::Color::TRANSPARENT,
                        }
                    };
                    button::Style {
                        background: Some(iced::Background::Color(bg)),
                        text_color: theme::TEXT_PRIMARY,
                        border: iced::Border::default(),
                        shadow: iced::Shadow::default(),
                        snap: false,
                    }
                });

            if open_menu.is_some() {
                mouse_area(btn)
                    .on_enter(MenuBarMessage::TopLevelHovered(i))
                    .into()
            } else {
                btn.into()
            }
        })
        .collect();

    container(
        row(items)
            .spacing(0)
            .align_y(Alignment::Center)
            .padding(Padding::from([2.0, 4.0])),
    )
    .width(Fill)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::HEADER_BG)),
        ..Default::default()
    })
    .into()
}

/// Draw the floating dropdown + optional submenu panel.
pub fn dropdown<'a>(
    open_menu: Option<usize>,
    open_submenu: Option<usize>,
    state: &MenuState,
) -> Option<Element<'a, MenuBarMessage>> {
    let index = open_menu?;
    let (_, entries) = menu::MENUS.get(index)?;

    // Render main dropdown items (no inline submenus)
    let items = render_entries(entries, index, open_submenu, state);
    let main_panel = styled_panel(column(items).spacing(2).width(DROPDOWN_WIDTH));

    // If a submenu is open, render it as a separate panel
    let panels: Element<'_, MenuBarMessage> = if let Some(sub_idx) = open_submenu {
        if let Some(entry) = entries.get(sub_idx) {
            let sub_entries = entry.sub_items();
            if !sub_entries.is_empty() {
                // Calculate vertical offset: approximate height of items above the submenu entry
                let v_offset: f32 = entries[..sub_idx]
                    .iter()
                    .map(|e| if e.is_separator() { 9.0 } else { ITEM_PADDING.top + ITEM_PADDING.bottom + 16.0 })
                    .sum::<f32>()
                    + 6.0; // panel top padding

                let sub_items: Vec<Element<'_, MenuBarMessage>> = sub_entries
                    .iter()
                    .enumerate()
                    .map(|(sub_i, sub_entry)| {
                        render_submenu_item(sub_entry, index, sub_idx, sub_i, state)
                    })
                    .collect();

                let sub_panel = styled_panel(column(sub_items).spacing(2).width(SUBMENU_WIDTH));

                let sub_with_offset = column![
                    iced::widget::Space::new().height(v_offset),
                    sub_panel,
                ];

                return Some(build_offset_row(index, row![main_panel, sub_with_offset].spacing(4)));
            }
        }
        build_offset_row(index, main_panel)
    } else {
        build_offset_row(index, main_panel)
    };

    Some(panels)
}

/// Wrap content in a styled dropdown panel.
fn styled_panel<'a>(content: impl Into<Element<'a, MenuBarMessage>>) -> Element<'a, MenuBarMessage> {
    container(content)
        .padding(Padding::from([6.0, 0.0]))
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::CARD_BG)),
            border: iced::Border {
                color: theme::BORDER,
                width: 1.0,
                radius: PANEL_RADIUS.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Build a row with invisible spacers to align dropdown under the correct menu label.
fn build_offset_row<'a>(
    menu_index: usize,
    content: impl Into<Element<'a, MenuBarMessage>>,
) -> Element<'a, MenuBarMessage> {
    let mut row_items: Vec<Element<'_, MenuBarMessage>> = Vec::new();
    for (i, (label, _)) in menu::MENUS.iter().enumerate() {
        if i == menu_index {
            row_items.push(content.into());
            break;
        }
        row_items.push(
            container(text(*label).size(13).color(iced::Color::TRANSPARENT))
                .padding(Padding::from([4.0, 10.0]))
                .into(),
        );
    }
    row(row_items)
        .spacing(0)
        .padding(Padding::from([0.0, 4.0]))
        .into()
}

fn render_entries<'a>(
    entries: &[MenuEntry],
    menu_index: usize,
    open_submenu: Option<usize>,
    state: &MenuState,
) -> Vec<Element<'a, MenuBarMessage>> {
    entries
        .iter()
        .enumerate()
        .map(|(item_i, entry)| render_entry(entry, menu_index, item_i, open_submenu, state))
        .collect()
}

fn render_entry<'a>(
    entry: &MenuEntry,
    menu_index: usize,
    item_index: usize,
    open_submenu: Option<usize>,
    _state: &MenuState,
) -> Element<'a, MenuBarMessage> {
    if entry.is_separator() {
        return container(rule::horizontal(1).style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }))
        .padding(Padding::from([2.0, 8.0]))
        .into();
    }

    let enabled = entry.is_enabled(_state);
    let is_sub = entry.is_submenu();
    let sub_is_open = is_sub && open_submenu == Some(item_index);

    let label_color = if enabled { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };
    let shortcut_color = if enabled { theme::TEXT_SECONDARY } else { theme::TEXT_MUTED };

    let label_text = text(entry.label()).size(13).color(label_color);

    let content: Element<'_, MenuBarMessage> = if is_sub {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            crate::icons::CHEVRON_RIGHT.render(11.0, shortcut_color),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    } else {
        let shortcut = entry.shortcut_text();
        if !shortcut.is_empty() {
            row![
                label_text,
                iced::widget::Space::new().width(Fill),
                text(shortcut).size(11).color(shortcut_color),
            ]
            .spacing(16)
            .align_y(Alignment::Center)
            .into()
        } else {
            label_text.into()
        }
    };

    let mut btn = button(content)
        .padding(ITEM_PADDING)
        .width(Fill)
        .style(move |_theme, status| {
            let bg = if sub_is_open {
                theme::ITEM_HOVER
            } else if enabled {
                match status {
                    button::Status::Hovered => theme::ITEM_HOVER,
                    _ => iced::Color::TRANSPARENT,
                }
            } else {
                iced::Color::TRANSPARENT
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: label_color,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    if enabled {
        btn = btn.on_press(MenuBarMessage::ItemClicked(menu_index, item_index));
    }

    // Wrap in mouse_area for hover-based submenu opening / closing
    if is_sub && enabled {
        mouse_area(btn)
            .on_enter(MenuBarMessage::SubMenuHovered(menu_index, item_index))
            .into()
    } else if enabled {
        mouse_area(btn)
            .on_enter(MenuBarMessage::SubMenuHovered(menu_index, usize::MAX))
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
) -> Element<'a, MenuBarMessage> {
    if entry.is_separator() {
        return container(rule::horizontal(1).style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }))
        .padding(Padding::from([2.0, 8.0]))
        .into();
    }

    let enabled = entry.is_enabled(state);
    let label_color = if enabled { theme::TEXT_PRIMARY } else { theme::TEXT_MUTED };

    let label_text = text(entry.label()).size(13).color(label_color);

    let shortcut = entry.shortcut_text();
    let content: Element<'_, MenuBarMessage> = if !shortcut.is_empty() {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            text(shortcut)
                .size(11)
                .color(if enabled { theme::TEXT_SECONDARY } else { theme::TEXT_MUTED }),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    } else {
        label_text.into()
    };

    let mut btn = button(content)
        .padding(ITEM_PADDING)
        .width(Fill)
        .style(move |_theme, status| {
            let bg = if enabled {
                match status {
                    button::Status::Hovered => theme::ITEM_HOVER,
                    _ => iced::Color::TRANSPARENT,
                }
            } else {
                iced::Color::TRANSPARENT
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: label_color,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        });

    if enabled {
        btn = btn.on_press(MenuBarMessage::SubMenuItemClicked(
            menu_index,
            parent_index,
            sub_index,
        ));
    }

    btn.into()
}

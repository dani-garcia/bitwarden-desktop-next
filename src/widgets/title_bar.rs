use iced::widget::{button, column, container, mouse_area, row, rule, text};
use iced::{Alignment, Element, Fill, Padding};

use crate::menu::{self, MenuEntry, MenuState};
use crate::theme;

const DROPDOWN_WIDTH: f32 = 280.0;
const SUBMENU_WIDTH: f32 = 220.0;
const ITEM_PADDING: Padding = Padding {
    top: 8.0,
    right: 16.0,
    bottom: 8.0,
    left: 16.0,
};
const PANEL_RADIUS: f32 = 8.0;
const TITLE_BAR_HEIGHT: f32 = 32.0;
const WINDOW_BTN_WIDTH: f32 = 46.0;

#[derive(Debug, Clone)]
pub enum TitleBarMessage {
    // Menu messages
    TopLevelClicked(usize),
    TopLevelHovered(usize),
    ItemClicked(usize, usize),
    SubMenuHovered(usize, usize),
    SubMenuItemClicked(usize, usize, usize),
    // Window control messages
    MinimizeClicked,
    MaximizeClicked,
    CloseClicked,
    DragStart,
    ResizeEdge(iced::window::Direction),
}

/// Title bar height, exported for dropdown offset positioning.
pub const fn height() -> f32 {
    TITLE_BAR_HEIGHT
}

/// Draw the title bar: menu labels on the left, window buttons on the right.
pub fn view<'a>(open_menu: Option<usize>, is_maximized: bool) -> Element<'a, TitleBarMessage> {
    let menu_items: Vec<Element<'_, TitleBarMessage>> = menu::MENUS
        .iter()
        .enumerate()
        .map(|(i, (label, _))| {
            let is_open = open_menu == Some(i);
            let btn = button(text(*label).size(13).color(theme::TEXT_PRIMARY))
                .on_press(TitleBarMessage::TopLevelClicked(i))
                .padding([4, 10])
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
                    .on_enter(TitleBarMessage::TopLevelHovered(i))
                    .into()
            } else {
                btn.into()
            }
        })
        .collect();

    let menu_row = row(menu_items).spacing(0).align_y(Alignment::Center);

    // Window control buttons — native Segoe MDL2 on Windows, Bootstrap Icons on Linux
    let minimize_btn = window_chrome_button(
        chrome_icon::MINIMIZE,
        chrome_icon::FONT,
        theme::TITLEBAR_BTN_HOVER,
        TitleBarMessage::MinimizeClicked,
    );
    let max_icon = if is_maximized {
        chrome_icon::RESTORE
    } else {
        chrome_icon::MAXIMIZE
    };
    let maximize_btn = window_chrome_button(
        max_icon,
        chrome_icon::FONT,
        theme::TITLEBAR_BTN_HOVER,
        TitleBarMessage::MaximizeClicked,
    );
    let close_btn = window_chrome_button(
        chrome_icon::CLOSE,
        chrome_icon::FONT,
        theme::TITLEBAR_CLOSE_HOVER,
        TitleBarMessage::CloseClicked,
    );

    let bar_content = row![
        menu_row,
        iced::widget::Space::new().width(Fill),
        minimize_btn,
        maximize_btn,
        close_btn,
    ]
    .spacing(0)
    .align_y(Alignment::Center)
    .padding(Padding {
        top: 0.0,
        right: 0.0,
        bottom: 0.0,
        left: 4.0,
    });

    let bar = container(bar_content)
        .width(Fill)
        .height(TITLE_BAR_HEIGHT)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::HEADER_BG)),
            ..Default::default()
        });

    // Make the title bar draggable (buttons inside capture their own clicks)
    mouse_area(bar).on_press(TitleBarMessage::DragStart).into()
}

/// Platform-specific window chrome icons.
mod chrome_icon {
    #[cfg(target_os = "windows")]
    pub const FONT: iced::Font = iced::Font {
        family: iced::font::Family::Name("Segoe MDL2 Assets"),
        weight: iced::font::Weight::Normal,
        stretch: iced::font::Stretch::Normal,
        style: iced::font::Style::Normal,
    };
    #[cfg(target_os = "windows")]
    pub const MINIMIZE: char = '\u{E921}'; // ChromeMinimize
    #[cfg(target_os = "windows")]
    pub const MAXIMIZE: char = '\u{E922}'; // ChromeMaximize
    #[cfg(target_os = "windows")]
    pub const CLOSE: char = '\u{E8BB}'; // ChromeClose
    #[cfg(target_os = "windows")]
    pub const RESTORE: char = '\u{E923}'; // ChromeRestore

    #[cfg(not(target_os = "windows"))]
    pub const FONT: iced::Font = crate::icons::FONT;
    #[cfg(not(target_os = "windows"))]
    pub const MINIMIZE: char = crate::icons::DASH_LG.char();
    #[cfg(not(target_os = "windows"))]
    pub const MAXIMIZE: char = crate::icons::SQUARE.char();
    #[cfg(not(target_os = "windows"))]
    pub const CLOSE: char = crate::icons::X_LG.char();
    #[cfg(not(target_os = "windows"))]
    pub const RESTORE: char = crate::icons::WINDOW_STACK.char();
}

/// A flat window control button using a platform-specific icon font glyph.
fn window_chrome_button<'a>(
    glyph: char,
    font: iced::Font,
    hover_color: iced::Color,
    message: TitleBarMessage,
) -> Element<'a, TitleBarMessage> {
    let icon = container(text(glyph).font(font).size(10).color(theme::TEXT_PRIMARY))
        .width(Fill)
        .height(Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center);

    button(icon)
        .on_press(message)
        .width(WINDOW_BTN_WIDTH)
        .height(TITLE_BAR_HEIGHT)
        .padding(0)
        .style(move |_theme, status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => hover_color,
                _ => iced::Color::TRANSPARENT,
            };
            button::Style {
                background: Some(iced::Background::Color(bg)),
                text_color: theme::TEXT_PRIMARY,
                border: iced::Border::default(),
                shadow: iced::Shadow::default(),
                snap: false,
            }
        })
        .into()
}

/// Wrap content with invisible resize handles overlaid on all edges and corners.
/// Uses stack! so handles don't take up space or create visible borders.
pub fn resize_wrapper<'a, M: Clone + 'a>(
    content: impl Into<Element<'a, M>>,
    map_direction: fn(iced::window::Direction) -> M,
) -> Element<'a, M> {
    use iced::Length::Fixed;
    use iced::mouse;
    use iced::window::Direction;

    const EDGE: f32 = 5.0;
    const CORNER: f32 = 8.0;

    let content = content.into();

    let handle = |w: f32, h: f32, dir: Direction, cursor: mouse::Interaction| -> Element<'a, M> {
        mouse_area(iced::widget::Space::new().width(w).height(h))
            .on_press(map_direction(dir))
            .interaction(cursor)
            .into()
    };

    // Top edge: [NW corner] [N edge fill] [NE corner]
    let top_row: Element<'a, M> = row![
        handle(
            CORNER,
            EDGE,
            Direction::NorthWest,
            mouse::Interaction::ResizingDiagonallyDown
        ),
        mouse_area(iced::widget::Space::new().width(Fill).height(Fixed(EDGE)))
            .on_press(map_direction(Direction::North))
            .interaction(mouse::Interaction::ResizingVertically),
        handle(
            CORNER,
            EDGE,
            Direction::NorthEast,
            mouse::Interaction::ResizingDiagonallyUp
        ),
    ]
    .spacing(0)
    .into();

    // Bottom edge: [SW corner] [S edge fill] [SE corner]
    let bottom_row: Element<'a, M> = container(
        row![
            handle(
                CORNER,
                EDGE,
                Direction::SouthWest,
                mouse::Interaction::ResizingDiagonallyUp
            ),
            mouse_area(iced::widget::Space::new().width(Fill).height(Fixed(EDGE)))
                .on_press(map_direction(Direction::South))
                .interaction(mouse::Interaction::ResizingVertically),
            handle(
                CORNER,
                EDGE,
                Direction::SouthEast,
                mouse::Interaction::ResizingDiagonallyDown
            ),
        ]
        .spacing(0),
    )
    .align_y(iced::alignment::Vertical::Bottom)
    .width(Fill)
    .height(Fill)
    .into();

    // Left edge (between corners)
    let left_edge: Element<'a, M> = container(
        mouse_area(iced::widget::Space::new().width(Fixed(EDGE)).height(Fill))
            .on_press(map_direction(Direction::West))
            .interaction(mouse::Interaction::ResizingHorizontally),
    )
    .width(Fill)
    .height(Fill)
    .padding(Padding {
        top: CORNER,
        right: 0.0,
        bottom: CORNER,
        left: 0.0,
    })
    .into();

    // Right edge (between corners)
    let right_edge: Element<'a, M> = container(
        mouse_area(iced::widget::Space::new().width(Fixed(EDGE)).height(Fill))
            .on_press(map_direction(Direction::East))
            .interaction(mouse::Interaction::ResizingHorizontally),
    )
    .width(Fill)
    .height(Fill)
    .align_x(iced::alignment::Horizontal::Right)
    .padding(Padding {
        top: CORNER,
        right: 0.0,
        bottom: CORNER,
        left: 0.0,
    })
    .into();

    iced::widget::stack![content, top_row, bottom_row, left_edge, right_edge].into()
}

/// Draw the floating dropdown + optional submenu panel.
pub fn dropdown<'a>(
    open_menu: Option<usize>,
    open_submenu: Option<usize>,
    state: &MenuState,
) -> Option<Element<'a, TitleBarMessage>> {
    let index = open_menu?;
    let (_, entries) = menu::MENUS.get(index)?;

    let items = render_entries(entries, index, open_submenu, state);
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

        let sub_items: Vec<Element<'_, TitleBarMessage>> = entry
            .children
            .iter()
            .enumerate()
            .map(|(sub_i, sub_entry)| render_submenu_item(sub_entry, index, sub_idx, sub_i, state))
            .collect();

        let sub_panel = styled_panel(column(sub_items).spacing(2).width(SUBMENU_WIDTH));

        let sub_with_offset = column![iced::widget::Space::new().height(v_offset), sub_panel,];

        return Some(build_offset_row(
            index,
            row![main_panel, sub_with_offset].spacing(4),
        ));
    }

    Some(build_offset_row(index, main_panel))
}

fn styled_panel<'a>(
    content: impl Into<Element<'a, TitleBarMessage>>,
) -> Element<'a, TitleBarMessage> {
    container(content)
        .padding([6, 0])
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

fn build_offset_row<'a>(
    menu_index: usize,
    content: impl Into<Element<'a, TitleBarMessage>>,
) -> Element<'a, TitleBarMessage> {
    let mut row_items: Vec<Element<'_, TitleBarMessage>> = Vec::new();
    for (i, (label, _)) in menu::MENUS.iter().enumerate() {
        if i == menu_index {
            row_items.push(content.into());
            break;
        }
        row_items.push(
            container(text(*label).size(13).color(iced::Color::TRANSPARENT))
                .padding([4, 10])
                .into(),
        );
    }
    row(row_items).spacing(0).padding([0, 4]).into()
}

fn render_entries<'a>(
    entries: &[MenuEntry],
    menu_index: usize,
    open_submenu: Option<usize>,
    state: &MenuState,
) -> Vec<Element<'a, TitleBarMessage>> {
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
    state: &MenuState,
) -> Element<'a, TitleBarMessage> {
    if entry.is_separator() {
        return container(rule::horizontal(1).style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }))
        .padding([2, 8])
        .into();
    }

    let enabled = entry.is_enabled(state);
    let is_sub = entry.is_submenu();
    let sub_is_open = is_sub && open_submenu == Some(item_index);

    let label_color = if enabled {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };
    let shortcut_color = if enabled {
        theme::TEXT_SECONDARY
    } else {
        theme::TEXT_MUTED
    };

    let label_text = text(entry.label).size(13).color(label_color);

    let content: Element<'_, TitleBarMessage> = if is_sub {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            crate::icons::CHEVRON_RIGHT.render(11.0, shortcut_color),
        ]
        .spacing(16)
        .align_y(Alignment::Center)
        .into()
    } else if let Some(shortcut_text) = entry.shortcut_display() {
        row![
            label_text,
            iced::widget::Space::new().width(Fill),
            text(shortcut_text).size(11).color(shortcut_color),
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
) -> Element<'a, TitleBarMessage> {
    if entry.is_separator() {
        return container(rule::horizontal(1).style(|_theme| rule::Style {
            color: theme::BORDER,
            radius: 0.0.into(),
            fill_mode: rule::FillMode::Full,
            snap: false,
        }))
        .padding([2, 8])
        .into();
    }

    let enabled = entry.is_enabled(state);
    let label_color = if enabled {
        theme::TEXT_PRIMARY
    } else {
        theme::TEXT_MUTED
    };

    let label_text = text(entry.label).size(13).color(label_color);

    let content: Element<'_, TitleBarMessage> =
        if let Some(shortcut_text) = entry.shortcut_display() {
            row![
                label_text,
                iced::widget::Space::new().width(Fill),
                text(shortcut_text).size(11).color(if enabled {
                    theme::TEXT_SECONDARY
                } else {
                    theme::TEXT_MUTED
                }),
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
        btn = btn.on_press(TitleBarMessage::SubMenuItemClicked(
            menu_index,
            parent_index,
            sub_index,
        ));
    }

    btn.into()
}

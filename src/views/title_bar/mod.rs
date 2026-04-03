mod dropdown;
pub mod window_chrome;

use iced::{
    Alignment, Background, Border, Color, Element, Fill, Padding, Shadow,
    widget::{button, container, mouse_area, row, text},
};

use crate::{
    menu,
    theme::{AppColors, AppTheme},
};

use self::window_chrome::{chrome_button, icon as chrome_icon};

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
    DismissMenu,
    // Window control messages
    MinimizeClicked,
    MaximizeClicked,
    CloseClicked,
    DragStart,
    ResizeEdge(iced::window::Direction),
}

/// Wrap content with invisible resize handles on all edges.
pub use self::window_chrome::resize_wrapper;

/// Draw the title bar: menu labels on the left, window buttons on the right.
/// Each menu label wraps a `DropDown` that shows its panel via iced's overlay system.
pub fn view<'a>(
    open_menu: Option<usize>,
    open_submenu: Option<usize>,
    is_maximized: bool,
    menu_state: &menu::MenuState,
    colors: &AppColors,
) -> Element<'a, TitleBarMessage, AppTheme> {
    let text_primary = colors.text_primary;

    let menu_items: Vec<Element<'_, TitleBarMessage, AppTheme>> = menu::MENUS
        .iter()
        .enumerate()
        .map(|(i, (label, entries))| {
            let is_open = open_menu == Some(i);
            let btn = button(text(*label).size(13).color(text_primary))
                .on_press(TitleBarMessage::TopLevelClicked(i))
                .padding([4, 10])
                .style(move |theme: &AppTheme, status| {
                    let bg = if is_open {
                        theme.colors.card_bg
                    } else {
                        match status {
                            button::Status::Hovered => theme.colors.item_hover,
                            _ => Color::TRANSPARENT,
                        }
                    };
                    button::Style {
                        background: Some(Background::Color(bg)),
                        text_color: theme.colors.text_primary,
                        border: Border::default(),
                        shadow: Shadow::default(),
                        snap: false,
                    }
                });

            let panel =
                dropdown::menu_panel(entries, i, open_submenu, menu_state, colors);
            let dd: Element<'_, TitleBarMessage, AppTheme> =
                iced_aw::DropDown::new(btn, panel, is_open)
                    .on_dismiss(TitleBarMessage::DismissMenu)
                    .alignment(iced_aw::drop_down::Alignment::Bottom)
                    .width(iced::Length::Shrink)
                    .offset(0.0)
                    .into();

            if open_menu.is_some() {
                mouse_area(dd)
                    .on_enter(TitleBarMessage::TopLevelHovered(i))
                    .into()
            } else {
                dd
            }
        })
        .collect();

    let menu_row = row(menu_items).spacing(0).align_y(Alignment::Center);

    // Window control buttons
    let minimize_btn = chrome_button(
        chrome_icon::MINIMIZE,
        chrome_icon::FONT,
        colors.titlebar_btn_hover,
        TitleBarMessage::MinimizeClicked,
    );
    let max_icon = if is_maximized {
        chrome_icon::RESTORE
    } else {
        chrome_icon::MAXIMIZE
    };
    let maximize_btn = chrome_button(
        max_icon,
        chrome_icon::FONT,
        colors.titlebar_btn_hover,
        TitleBarMessage::MaximizeClicked,
    );
    let close_btn = chrome_button(
        chrome_icon::CLOSE,
        chrome_icon::FONT,
        colors.titlebar_close_hover,
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
        .style(|theme: &AppTheme| container::Style {
            background: Some(Background::Color(theme.colors.header_bg)),
            ..Default::default()
        });

    mouse_area(bar).on_press(TitleBarMessage::DragStart).into()
}

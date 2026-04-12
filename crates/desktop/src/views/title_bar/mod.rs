mod dropdown;
pub mod window_chrome;

use iced::{
    Alignment, Background, Border, Color, Element, Fill, Padding, Shadow, Task,
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

// ── Events ─────────────────────────────────────────────────────────────────
//
// Declarative facts the title bar bubbles up. The App router translates them
// to concrete window operations or menu actions.

#[derive(Debug, Clone, Copy)]
pub enum WindowCommand {
    Minimize,
    Maximize,
    Close,
    Drag,
    ResizeEdge(iced::window::Direction),
}

#[derive(Debug, Clone)]
pub enum TitleBarEvent {
    /// User invoked a menu entry mapping to a global `MenuAction`.
    MenuInvoked(crate::menu::MenuAction),
    /// User clicked a window-chrome button or started a drag/resize.
    Window(WindowCommand),
}

pub struct TitleBarState {
    pub open_menu: Option<usize>,
    pub open_submenu: Option<usize>,
}

impl TitleBarState {
    pub fn new() -> Self {
        Self {
            open_menu: None,
            open_submenu: None,
        }
    }

    pub fn dismiss_menu(&mut self) {
        self.open_menu = None;
        self.open_submenu = None;
    }

    /// Compositional MVU update. Title bar has no async work so the returned
    /// task is always `Task::none()`; the event carries the domain fact.
    pub fn update(
        &mut self,
        msg: TitleBarMessage,
    ) -> (Task<TitleBarMessage>, Option<TitleBarEvent>) {
        match msg {
            TitleBarMessage::TopLevelClicked(i) => {
                self.open_menu = if self.open_menu == Some(i) {
                    None
                } else {
                    Some(i)
                };
                self.open_submenu = None;
                (Task::none(), None)
            }
            TitleBarMessage::DismissMenu => {
                self.dismiss_menu();
                (Task::none(), None)
            }
            TitleBarMessage::TopLevelHovered(i) => {
                self.open_menu = Some(i);
                self.open_submenu = None;
                (Task::none(), None)
            }
            TitleBarMessage::SubMenuHovered(_menu, item) => {
                self.open_submenu = if item == usize::MAX { None } else { Some(item) };
                (Task::none(), None)
            }
            TitleBarMessage::ItemClicked(menu, item) => {
                self.dismiss_menu();
                let event = crate::menu::MENUS
                    .get(menu)
                    .and_then(|(_, entries)| entries.get(item))
                    .and_then(|e| e.action)
                    .map(TitleBarEvent::MenuInvoked);
                (Task::none(), event)
            }
            TitleBarMessage::SubMenuItemClicked(menu, parent, sub) => {
                self.dismiss_menu();
                let event = crate::menu::MENUS
                    .get(menu)
                    .and_then(|(_, entries)| entries.get(parent))
                    .and_then(|e| e.children.get(sub))
                    .and_then(|e| e.action)
                    .map(TitleBarEvent::MenuInvoked);
                (Task::none(), event)
            }
            TitleBarMessage::MinimizeClicked => (
                Task::none(),
                Some(TitleBarEvent::Window(WindowCommand::Minimize)),
            ),
            TitleBarMessage::MaximizeClicked => (
                Task::none(),
                Some(TitleBarEvent::Window(WindowCommand::Maximize)),
            ),
            TitleBarMessage::CloseClicked => (
                Task::none(),
                Some(TitleBarEvent::Window(WindowCommand::Close)),
            ),
            TitleBarMessage::DragStart => (
                Task::none(),
                Some(TitleBarEvent::Window(WindowCommand::Drag)),
            ),
            TitleBarMessage::ResizeEdge(dir) => (
                Task::none(),
                Some(TitleBarEvent::Window(WindowCommand::ResizeEdge(dir))),
            ),
        }
    }
}

/// Wrap content with invisible resize handles on all edges.
pub use self::window_chrome::resize_wrapper;

impl TitleBarState {
    /// Draw a minimal title bar with no buttons or drag area (used on macOS
    /// when the native system title bar takes over). Takes no state so it's
    /// an associated function, not a `&self` method.
    pub fn view_empty<'a>() -> Element<'a, TitleBarMessage, AppTheme> {
        container(iced::widget::Space::new())
            .width(Fill)
            .height(TITLE_BAR_HEIGHT)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.header_bg)),
                ..Default::default()
            })
            .into()
    }

    /// Draw the title bar: menu labels on the left, window buttons on the right.
    /// Each menu label wraps a `DropDown` that shows its panel via iced's overlay
    /// system. `is_maximized` and `menu_state` come from App (window-level state
    /// and derived app-state respectively); the open menu + submenu come from `self`.
    pub fn view<'a>(
        &self,
        is_maximized: bool,
        menu_state: &menu::MenuState,
        colors: &AppColors,
    ) -> Element<'a, TitleBarMessage, AppTheme> {
        let open_menu = self.open_menu;
        let open_submenu = self.open_submenu;

        let menu_items: Vec<Element<'_, TitleBarMessage, AppTheme>> = menu::MENUS
            .iter()
            .enumerate()
            .map(|(i, (label, entries))| {
                let is_open = open_menu == Some(i);
                let btn = button(text(*label).size(14).color(Color::WHITE))
                    .on_press(TitleBarMessage::TopLevelClicked(i))
                    .padding([4, 10])
                    .style(move |theme: &AppTheme, status| {
                        let bg = if is_open {
                            theme.colors.titlebar_btn_hover
                        } else {
                            match status {
                                button::Status::Hovered => theme.colors.titlebar_btn_hover,
                                _ => Color::TRANSPARENT,
                            }
                        };
                        button::Style {
                            background: Some(Background::Color(bg)),
                            text_color: Color::WHITE,
                            border: Border::default(),
                            shadow: Shadow::default(),
                            snap: false,
                        }
                    });

                let panel = dropdown::menu_panel(entries, i, open_submenu, menu_state, colors);
                let dd: Element<'_, TitleBarMessage, AppTheme> =
                    crate::components::drop_down::DropDown::new(btn, panel, is_open)
                        .on_dismiss(TitleBarMessage::DismissMenu)
                        .alignment(crate::components::drop_down::Alignment::BelowLeft)
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
}

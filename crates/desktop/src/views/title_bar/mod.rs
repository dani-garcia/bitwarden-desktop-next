mod dropdown;
mod handler;
pub(super) mod window_chrome;

use iced::{
    Alignment, Background, Border, Color, Element, Fill, Padding, Shadow,
    widget::{button, container, mouse_area, row, text},
};

use crate::{
    app::{Outcome, Overlay, ViewTypes},
    services::menu,
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
pub enum WindowAction {
    Minimize,
    Maximize,
    Close,
    Drag,
    ResizeEdge(iced::window::Direction),
}

#[derive(Debug, Clone)]
pub enum TitleBarEvent {
    /// User invoked a menu entry mapping to a global `MenuAction`.
    MenuInvoked(crate::services::menu::MenuAction),
    /// User clicked a window-chrome button or started a drag/resize.
    Window(WindowAction),
}

/// Stateless view — menu open/close lives in the app-level [`Overlay`] cell.
/// The struct is kept (rather than making everything free functions) so the
/// compositional MVU pattern stays uniform across views.
pub struct TitleBarView;

impl ViewTypes for TitleBarView {
    type Message = TitleBarMessage;
    type Event = TitleBarEvent;
}

impl TitleBarView {
    pub fn new() -> Self {
        Self
    }

    /// Compositional MVU update. Title bar has no async work so the returned
    /// task is always `Task::none()`; the event carries the domain fact.
    pub fn update(
        &mut self,
        msg: TitleBarMessage,
        ctx: crate::app::UpdateCtx<'_>,
    ) -> Outcome<Self> {
        // Read the previously open top-level menu, if any, so toggle-on-
        // same-index can close and clicks on a different index can switch.
        let current_menu = match *ctx.open_overlay {
            Some(Overlay::TitleBarMenu { menu, .. }) => Some(menu),
            _ => None,
        };

        // Menu-navigation arms mutate overlay state; button-click arms bubble
        // a `TitleBarEvent::Window(...)` action.
        let window_action = match msg {
            TitleBarMessage::TopLevelClicked(i) => {
                *ctx.open_overlay = if current_menu == Some(i) {
                    None
                } else {
                    Some(Overlay::TitleBarMenu {
                        menu: i,
                        submenu: None,
                    })
                };
                return Outcome::None;
            }
            TitleBarMessage::DismissMenu => {
                *ctx.open_overlay = None;
                return Outcome::None;
            }
            TitleBarMessage::TopLevelHovered(i) => {
                *ctx.open_overlay = Some(Overlay::TitleBarMenu {
                    menu: i,
                    submenu: None,
                });
                return Outcome::None;
            }
            TitleBarMessage::SubMenuHovered(menu, item) => {
                let submenu = if item == usize::MAX { None } else { Some(item) };
                *ctx.open_overlay = Some(Overlay::TitleBarMenu { menu, submenu });
                return Outcome::None;
            }
            TitleBarMessage::ItemClicked(menu, item) => {
                *ctx.open_overlay = None;
                return Outcome::from_option(
                    crate::services::menu::MENUS
                        .get(menu)
                        .and_then(|(_, entries)| entries.get(item))
                        .and_then(|e| e.action)
                        .map(TitleBarEvent::MenuInvoked),
                );
            }
            TitleBarMessage::SubMenuItemClicked(menu, parent, sub) => {
                *ctx.open_overlay = None;
                return Outcome::from_option(
                    crate::services::menu::MENUS
                        .get(menu)
                        .and_then(|(_, entries)| entries.get(parent))
                        .and_then(|e| e.children.get(sub))
                        .and_then(|e| e.action)
                        .map(TitleBarEvent::MenuInvoked),
                );
            }
            TitleBarMessage::MinimizeClicked => WindowAction::Minimize,
            TitleBarMessage::MaximizeClicked => WindowAction::Maximize,
            TitleBarMessage::CloseClicked => WindowAction::Close,
            TitleBarMessage::DragStart => WindowAction::Drag,
            TitleBarMessage::ResizeEdge(dir) => WindowAction::ResizeEdge(dir),
        };
        Outcome::event(TitleBarEvent::Window(window_action))
    }
}

/// Wrap content with invisible resize handles on all edges.
pub use self::window_chrome::resize_wrapper;

impl TitleBarView {
    /// Draw a minimal title bar with no buttons or drag area (used on macOS
    /// when the native system title bar takes over). Takes no state so it's
    /// an associated function, not a `&self` method.
    pub fn view_empty<'a>() -> Element<'a, TitleBarMessage, AppTheme> {
        container(iced::widget::Space::new())
            .width(Fill)
            .height(TITLE_BAR_HEIGHT)
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.header_bg)
            })
            .into()
    }

    /// Draw the title bar: menu labels on the left, window buttons on the right.
    /// Each menu label wraps a `DropDown` that shows its panel via iced's overlay
    /// system. `is_maximized` and `menu_state` come from App (window-level state
    /// and derived app-state respectively); open-menu and open-submenu are
    /// extracted from the app-level overlay cell by `App::view_main`.
    pub fn view<'a>(
        &self,
        is_maximized: bool,
        menu_state: &menu::MenuState,
        open_menu: Option<usize>,
        open_submenu: Option<usize>,
        colors: &AppColors,
    ) -> Element<'a, TitleBarMessage, AppTheme> {
        let menu_items: Vec<Element<'_, TitleBarMessage, AppTheme>> = menu::MENUS
            .iter()
            .enumerate()
            .map(|(i, (label_key, entries))| {
                let is_open = open_menu == Some(i);
                let btn = button(
                    text(crate::services::i18n::lookup(label_key))
                        .size(14)
                        .color(Color::WHITE),
                )
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
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.header_bg)
            });

        mouse_area(bar).on_press(TitleBarMessage::DragStart).into()
    }
}

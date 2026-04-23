use iced::Task;

use crate::{
    app::{App, Message},
    views::title_bar::TitleBarEvent,
};

impl App {
    pub(crate) fn handle_titlebar_event(&mut self, event: TitleBarEvent) -> Task<Message> {
        match event {
            TitleBarEvent::MenuInvoked(menu_action) => self.handle_menu_action(menu_action),
            TitleBarEvent::Window(action) => self.handle_window_action(action),
        }
    }
}

use iced::Task;

use crate::{
    app::{App, Message},
    services::menu::MenuAction,
    views::new_item_picker::NewItemPickerEvent,
};

impl App {
    pub(crate) fn handle_new_item_picker_event(
        &mut self,
        event: NewItemPickerEvent,
    ) -> Task<Message> {
        match event {
            NewItemPickerEvent::NewItem(t) => self.handle_menu_action(MenuAction::NewItem(t)),
            NewItemPickerEvent::NewFolder => self.handle_menu_action(MenuAction::NewFolder),
        }
    }

    /// Open the New-item picker modal. No-op when no user is active.
    pub(crate) fn open_new_item_picker(&mut self) -> Task<Message> {
        if self.require_active_user_and_close_overlay().is_none() {
            return Task::none();
        }
        self.views.new_item_picker.open();
        Task::none()
    }
}

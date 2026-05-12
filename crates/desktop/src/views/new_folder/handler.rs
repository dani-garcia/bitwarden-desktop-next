use iced::Task;

use crate::{
    app::{App, Message},
    services::sdk::ClientExt,
    views::new_folder::{NewFolderEvent, NewFolderMessage},
};

impl App {
    pub(crate) fn handle_new_folder_event(&mut self, event: NewFolderEvent) -> Task<Message> {
        match event {
            NewFolderEvent::Run(name) => self.perform_with_active_client(
                move |client| async move { client.create_folder(name).await.map(|_| ()) },
                |_uid, res| NewFolderMessage::Saved(res).into(),
            ),
        }
    }

    /// Open the New folder modal. No-op when no user is active.
    pub(crate) fn open_new_folder_modal(&mut self) -> Task<Message> {
        if self.require_active_user_and_close_overlay().is_none() {
            return Task::none();
        }
        self.views.new_folder.open();
        crate::components::fade_in_out::focus_after_open(
            crate::views::new_folder::NAME_FIELD_ID.clone(),
        )
    }
}

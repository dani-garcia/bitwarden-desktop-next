use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    services::sdk::ClientExt,
    views::new_folder::{NewFolderEvent, NewFolderMessage},
};

impl App {
    pub(crate) fn handle_new_folder_event(&mut self, event: NewFolderEvent) -> Task<Message> {
        match event {
            NewFolderEvent::Run(name) => self.perform_with_active_client(
                move |client| async move { client.create_folder(name).await.map(|_| ()) },
                |_uid, res| Message::new_folder(NewFolderMessage::Saved(res)),
            ),
            NewFolderEvent::ToastSuccess => {
                self.push_toast(Toast::success(fl!("new-folder-toast-success"), None));
                Task::none()
            }
            NewFolderEvent::ToastError(err) => {
                tracing::warn!(%err, "create folder failed");
                self.push_toast(Toast::error(
                    fl!("new-folder-toast-failed-body"),
                    Some(&fl!("new-folder-toast-failed-title")),
                ));
                Task::none()
            }
        }
    }

    /// Open the New folder modal. No-op when no user is active.
    pub(crate) fn open_new_folder_modal(&mut self) -> Task<Message> {
        if self.active_user.is_none() {
            return Task::none();
        }
        self.open_overlay = None;
        self.views.new_folder.open();
        Task::none()
    }
}

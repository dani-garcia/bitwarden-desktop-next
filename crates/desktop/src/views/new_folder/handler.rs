use std::sync::Arc;

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    services::sdk::ClientManager,
    views::new_folder::{NewFolderEvent, NewFolderMessage},
};

impl App {
    pub(crate) fn handle_new_folder_event(&mut self, event: NewFolderEvent) -> Task<Message> {
        match event {
            NewFolderEvent::Run(name) => {
                let Some(uid) = self.active_user else {
                    return Task::none();
                };
                let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
                Task::perform(
                    async move { mgr.create_folder(&uid, name).await.map(|_| ()) },
                    |res| Message::new_folder(NewFolderMessage::Saved(res)),
                )
            }
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

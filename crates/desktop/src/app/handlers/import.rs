use std::sync::Arc;

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    services::sdk::ClientManager,
    views::import::{ImportEvent, ImportMessage},
};

impl App {
    pub(crate) fn handle_import_event(&mut self, event: ImportEvent) -> Task<Message> {
        match event {
            ImportEvent::Unimplemented => {
                self.push_toast(Toast::info(fl!("import-toast-unimplemented"), None));
                Task::none()
            }
        }
    }

    /// Open the Import modal, seed the vault dropdown synchronously from the
    /// active user's organization list (already cached on the vault view),
    /// and spawn an async folder list fetch — same SDK call shape as the
    /// cipher edit form.
    pub(crate) fn open_import_modal(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.open_overlay = None;
        self.views.import.open();

        let orgs = self
            .views
            .vault
            .organizations_for(&uid)
            .map(|s| s.to_vec())
            .unwrap_or_default();
        self.views.import.set_organizations(&orgs);
        self.views
            .import
            .set_collections(self.client_manager.list_collections(&uid));

        let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
        Task::perform(
            async move {
                match mgr.list_folders(&uid).await {
                    Ok(folders) => folders.into_iter().map(|f| f.name).collect(),
                    Err(err) => {
                        tracing::warn!(%err, "failed to load folders for import modal");
                        Vec::new()
                    }
                }
            },
            |names| Message::import(ImportMessage::FoldersLoaded(names)),
        )
    }
}

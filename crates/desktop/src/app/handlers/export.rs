use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    views::export::ExportEvent,
};

impl App {
    pub(crate) fn handle_export_event(&mut self, event: ExportEvent) -> Task<Message> {
        match event {
            ExportEvent::Unimplemented => {
                self.push_toast(Toast::info(fl!("export-toast-unimplemented"), None));
                Task::none()
            }
        }
    }

    /// Open the Export modal, passing the active account's email so the
    /// "individual vault" banner can name the user.
    pub(crate) fn open_export_modal(&mut self) -> Task<Message> {
        if self.active_user.is_none() {
            return Task::none();
        }
        self.open_overlay = None;
        let email = self
            .active_account_entry()
            .map(|a| a.email.clone())
            .unwrap_or_default();
        self.views.export.open(email);
        Task::none()
    }
}

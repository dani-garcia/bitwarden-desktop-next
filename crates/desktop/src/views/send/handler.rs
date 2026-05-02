use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    fl,
    views::send::SendEvent,
};

impl App {
    pub(crate) fn handle_send_event(&mut self, event: SendEvent) -> Task<Message> {
        match event {
            SendEvent::AccountSwitcher(e) => self.handle_account_switcher_event(e),
            SendEvent::ToastRequested(t) => {
                self.push_toast(t);
                Task::none()
            }
            SendEvent::ItemSaved { uid } => {
                self.push_toast(Toast::success(fl!("send-toast-item-saved"), None));
                self.load_send_list_task(uid)
            }
            SendEvent::ItemDeleted { uid } => {
                self.push_toast(Toast::success(fl!("send-toast-item-deleted"), None));
                self.load_send_list_task(uid)
            }
            SendEvent::ClipboardCopyRequested {
                value,
                sensitivity,
                toast_label,
            } => self.copy_and_toast(value, sensitivity, toast_label),
        }
    }
}

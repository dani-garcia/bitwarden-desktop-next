use iced::Task;

use crate::{
    app::{App, Message},
    services::clipboard::Sensitivity,
    views::about::{self, AboutMessage},
};

impl App {
    pub(crate) fn handle_about_message(&mut self, msg: AboutMessage) -> Task<Message> {
        match msg {
            AboutMessage::CopyInfo => {
                self.clipboard
                    .copy(about::info_string(), Sensitivity::Normal);
                Task::none()
            }
            AboutMessage::Close => {
                if let Some(id) = self.about_window_id() {
                    iced::window::close(id)
                } else {
                    Task::none()
                }
            }
        }
    }
}

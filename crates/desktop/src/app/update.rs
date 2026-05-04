//! `App::update` — top-level message dispatch.

use iced::Task;

use super::{App, Message, UpdateCtx, ViewMessage};

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::About(m) => self.handle_about_message(m),
            Message::Window(m) => self.handle_window_message(m),
            Message::System(m) => self.handle_system_message(m),
            Message::Sidebar(m) => self.handle_sidebar_message(m),
            Message::Magnify(m) => self.handle_magnify_message(m),
            // No-op — the redraw the message triggers is the entire point.
            // Subscription rebuilds and unsubscribes once nothing's animating.
            Message::AnimationTick => Task::none(),
            Message::Favicon(crate::services::favicon::FaviconMessage::IconResolved {
                uid,
                hostname,
            }) => {
                // Arrival drives the redraw; the service has already mutated
                // its in-memory cache. Log at trace so a cold unlock with
                // thousands of icons doesn't spam RUST_LOG=info users.
                tracing::trace!(%uid, %hostname, "favicon resolved");
                Task::none()
            }
            Message::View(view_msg) => {
                let active_vault_filter = self.sidebar.active_vault_filter;
                let active_send_filter = self.sidebar.active_send_filter;
                let uctx = UpdateCtx {
                    client_manager: &mut self.client_manager,
                    active_user: self.active_user.as_ref(),
                    active_vault_filter,
                    active_send_filter,
                    open_overlay: &mut self.open_overlay,
                };
                match view_msg {
                    ViewMessage::Login(m) => {
                        let outcome = self.views.login.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::login,
                            |s, e| s.handle_login_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Vault(m) => {
                        let outcome = self.views.vault.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::vault,
                            |s, e| s.handle_vault_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Send(m) => {
                        let outcome = self.views.send.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::send,
                            |s, e| s.handle_send_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::TitleBar(m) => {
                        let outcome = self.views.title_bar.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::title_bar,
                            |s, e| s.handle_titlebar_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Settings(m) => {
                        let outcome = self.views.settings.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::settings,
                            |s, e| s.handle_settings_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Generator(m) => {
                        let outcome = self.views.generator.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::generator,
                            |s, e| s.handle_generator_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Import(m) => {
                        let outcome = self.views.import.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::import,
                            |s, e| s.handle_import_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::Export(m) => {
                        let outcome = self.views.export.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::export,
                            |s, e| s.handle_export_event(e),
                            App::push_toast,
                        )
                    }
                    ViewMessage::NewFolder(m) => {
                        let outcome = self.views.new_folder.update(m, uctx);
                        outcome.dispatch(
                            self,
                            Message::new_folder,
                            |s, e| s.handle_new_folder_event(e),
                            App::push_toast,
                        )
                    }
                }
            }
        }
    }
}

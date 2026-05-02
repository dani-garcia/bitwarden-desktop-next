use std::{path::PathBuf, sync::Arc};

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    domain::UserId,
    fl,
    paths::data_dir,
    services::sdk::ClientManager,
    views::export::{ExportEvent, ExportMessage},
};

impl App {
    pub(crate) fn handle_export_event(&mut self, event: ExportEvent) -> Task<Message> {
        match event {
            ExportEvent::Run(format) => {
                let Some(uid) = self.active_user else {
                    return Task::none();
                };
                let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
                let dir = data_dir();
                Task::perform(
                    async move { run_export(mgr, uid, format, dir).await },
                    |r| Message::export(ExportMessage::Completed(r)),
                )
            }
            ExportEvent::ToastSuccess(path) => {
                self.push_toast(Toast::success(
                    fl!("export-toast-success", path = path.as_str()),
                    None,
                ));
                Task::none()
            }
            ExportEvent::ToastError(err) => {
                tracing::error!(%err, "vault export failed");
                self.push_toast(Toast::error(
                    fl!("export-toast-failed-body"),
                    Some(&fl!("export-toast-failed-title")),
                ));
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

/// Call the SDK exporter and write the result to a timestamped file under
/// `data/`. Returns the absolute path on success. Sync `std::fs` is
/// acceptable here — vault exports are small.
async fn run_export(
    mgr: Arc<ClientManager>,
    uid: UserId,
    format: bitwarden_exporters::ExportFormat,
    dir: PathBuf,
) -> Result<String, String> {
    let extension = match format {
        bitwarden_exporters::ExportFormat::Csv => "csv",
        bitwarden_exporters::ExportFormat::Json
        | bitwarden_exporters::ExportFormat::EncryptedJson { .. } => "json",
    };
    let content = mgr.export_vault(&uid, format).await?;

    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let filename = format!(
        "bitwarden-export-{}.{}",
        chrono::Local::now().format("%Y%m%d-%H%M%S"),
        extension,
    );
    let path = dir.join(filename);
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

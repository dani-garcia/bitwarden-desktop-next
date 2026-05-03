use std::{path::PathBuf, sync::Arc};

use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    domain::UserId,
    fl,
    services::sdk::ClientManager,
    views::export::{ExportEvent, ExportMessage},
};

impl App {
    pub(crate) fn handle_export_event(&mut self, event: ExportEvent) -> Task<Message> {
        match event {
            // `uid` is captured from the validated user, not re-read from
            // `self.active_user` — guards against an account switch between
            // Confirm and the rfd dialog redirecting the export.
            ExportEvent::PickPathThenRun { uid, format } => {
                let mgr: Arc<ClientManager> = Arc::clone(&self.client_manager);
                let extension = extension_for(&format);
                let default_name = format!(
                    "bitwarden-export-{}.{}",
                    chrono::Local::now().format("%Y%m%d-%H%M%S"),
                    extension,
                );
                Task::perform(
                    async move { pick_and_export(mgr, uid, format, extension, default_name).await },
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
            ExportEvent::ToastInvalidMasterPassword => {
                self.push_toast(Toast::error(fl!("export-confirm-error"), None));
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

fn extension_for(format: &bitwarden_exporters::ExportFormat) -> &'static str {
    match format {
        bitwarden_exporters::ExportFormat::Csv => "csv",
        bitwarden_exporters::ExportFormat::Json
        | bitwarden_exporters::ExportFormat::EncryptedJson { .. } => "json",
    }
}

/// Open a native save dialog and run the export to the chosen path. Returns
/// `Ok(None)` if the user cancelled the dialog (silent close), `Ok(Some(path))`
/// after a successful write, or `Err(_)` on SDK / I/O failure.
///
/// `rfd::AsyncFileDialog` is the resolution to iced #1002 — the sync variant
/// blocks the iced event loop on Linux/Windows and deadlocks on macOS where
/// iced and rfd both want the main thread; the async one cooperates with our
/// `Task::perform` runtime.
async fn pick_and_export(
    mgr: Arc<ClientManager>,
    uid: UserId,
    format: bitwarden_exporters::ExportFormat,
    extension: &'static str,
    default_name: String,
) -> Result<Option<String>, String> {
    let Some(handle) = rfd::AsyncFileDialog::new()
        .set_file_name(&default_name)
        .add_filter(extension, &[extension])
        .save_file()
        .await
    else {
        return Ok(None);
    };
    let path: PathBuf = handle.path().to_path_buf();
    write_export(mgr, uid, format, path).await.map(Some)
}

/// Call the SDK exporter and write the result to the user-chosen file.
/// Sync `std::fs::write` is acceptable here — vault exports are small.
async fn write_export(
    mgr: Arc<ClientManager>,
    uid: UserId,
    format: bitwarden_exporters::ExportFormat,
    path: PathBuf,
) -> Result<String, String> {
    let content = mgr.export_vault(&uid, format).await?;
    std::fs::write(&path, content).map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

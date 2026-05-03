use std::path::PathBuf;

use bitwarden_core::OrganizationId;
use bitwarden_pm::PasswordManagerClient;
use iced::Task;

use crate::{
    app::{App, Message},
    components::toast::Toast,
    domain::UserId,
    fl,
    services::sdk::ClientExt,
    views::export::{ExportEvent, ExportMessage},
};

impl App {
    pub(crate) fn handle_export_event(&mut self, event: ExportEvent) -> Task<Message> {
        match event {
            // `uid` is captured from the validated user, not re-read from
            // `self.active_user` — guards against an account switch between
            // Confirm and the rfd dialog redirecting the export.
            ExportEvent::PickPathThenRun {
                uid,
                organization_id,
                format,
            } => {
                let extension = extension_for(&format);
                let default_name = format!(
                    "bitwarden-export-{}.{}",
                    chrono::Local::now().format("%Y%m%d-%H%M%S"),
                    extension,
                );
                self.perform_with_client(
                    uid,
                    move |client| {
                        pick_and_export(
                            client,
                            uid,
                            organization_id,
                            format,
                            extension,
                            default_name,
                        )
                    },
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
    /// personal-vault banner can name the user, and seeding the source-vault
    /// dropdown from the cached org snapshot held by `VaultView`.
    pub(crate) fn open_export_modal(&mut self) -> Task<Message> {
        let Some(uid) = self.active_user else {
            return Task::none();
        };
        self.open_overlay = None;
        let email = self
            .active_account_entry()
            .map(|a| a.email.clone())
            .unwrap_or_default();
        self.views.export.open(email);

        let orgs = self
            .views
            .vault
            .organizations_for(&uid)
            .map(|s| s.to_vec())
            .unwrap_or_default();
        self.views.export.set_organizations(&orgs);
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
    client: PasswordManagerClient,
    uid: UserId,
    organization_id: Option<OrganizationId>,
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
    write_export(client, uid, organization_id, format, path)
        .await
        .map(Some)
}

/// Call the SDK exporter and write the result to the user-chosen file.
/// `tokio::fs::write` so a slow target (network drive, USB) doesn't park
/// the runtime worker — large vaults can serialize to tens of MB.
async fn write_export(
    client: PasswordManagerClient,
    _uid: UserId,
    organization_id: Option<OrganizationId>,
    format: bitwarden_exporters::ExportFormat,
    path: PathBuf,
) -> Result<String, String> {
    let content = match organization_id {
        None => client.export_vault(format).await?,
        Some(org_id) => client.export_organization_vault(org_id, format).await?,
    };
    tokio::fs::write(&path, content)
        .await
        .map_err(|e| e.to_string())?;
    Ok(path.display().to_string())
}

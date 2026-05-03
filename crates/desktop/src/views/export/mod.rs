//! Export vault modal — wired to the sidebar Export button.
//!
//! Two stacked dialogs: the format/password picker (Compose), and a
//! master-password "Confirm vault export" gate that opens on top once the
//! user clicks Continue. Validation goes through the SDK
//! (`ClientManager::validate_master_password`); on success App is asked to
//! pop a native save dialog (rfd) and write the export to the chosen path.
//!
//! Format options mirror [`bitwarden_exporters::ExportFormat`] (Csv, Json,
//! EncryptedJson). Selecting `EncryptedJson` reveals a file-password input
//! below the picker; the encryption password is folded into the SDK-shaped
//! format value at the moment Continue is pressed.

use std::sync::Arc;

use iced::{
    Alignment, Element, Fill, Padding,
    widget::{self, Space, column, container, row, stack, text},
};

use crate::{
    app::{Outcome, UpdateCtx, ViewTypes},
    components::{FadeInOut, buttons, fade_in_out, icons, inputs, modal},
    domain::UserId,
    fl,
    services::sdk::ClientManager,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

const CONFIRM_PASSWORD_FIELD_ID: widget::Id = widget::Id::new("export-confirm-password-field");

// ── Export format catalog ─────────────────────────────────────────────────
//
// Mirrors the variants of [`bitwarden_exporters::ExportFormat`]. The SDK
// enum carries `password: String` inside `EncryptedJson`, but the picker
// only needs identity (which variant) — the password is collected from a
// separate input. `to_sdk` reassembles the SDK-shaped value at submit time.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormatChoice {
    Json,
    Csv,
    EncryptedJson,
}

impl ExportFormatChoice {
    const ALL: &'static [Self] = &[Self::Json, Self::Csv, Self::EncryptedJson];

    fn label(self) -> &'static str {
        match self {
            Self::Json => ".json",
            Self::Csv => ".csv",
            Self::EncryptedJson => ".json (Encrypted)",
        }
    }

    fn to_sdk(self, password: String) -> bitwarden_exporters::ExportFormat {
        match self {
            Self::Json => bitwarden_exporters::ExportFormat::Json,
            Self::Csv => bitwarden_exporters::ExportFormat::Csv,
            Self::EncryptedJson => bitwarden_exporters::ExportFormat::EncryptedJson { password },
        }
    }

    #[expect(
        dead_code,
        reason = "we keep this to ensure a compilation error when new SDK formats are added"
    )]
    fn from_sdk(format: &bitwarden_exporters::ExportFormat) -> Self {
        match format {
            bitwarden_exporters::ExportFormat::Json => Self::Json,
            bitwarden_exporters::ExportFormat::Csv => Self::Csv,
            bitwarden_exporters::ExportFormat::EncryptedJson { .. } => Self::EncryptedJson,
        }
    }
}

impl std::fmt::Display for ExportFormatChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ── State ─────────────────────────────────────────────────────────────────

pub struct ExportView {
    /// Outer fade — owns the Compose dialog (format picker).
    pub fade: FadeInOut,
    /// Inner fade — owns the Confirm dialog (master password). Opens only
    /// while the user is on the master-password gate.
    pub confirm_fade: FadeInOut,
    /// Active user email — shown verbatim in the "Only items associated with"
    /// banner. Set via [`Self::open`] each time the modal is opened.
    email: String,
    selected_format: ExportFormatChoice,
    /// Encryption password for the encrypted-json variant. Cleared on open
    /// and on successful submit. Only read when
    /// `selected_format == EncryptedJson`.
    file_password: String,
    /// Master-password input on the Confirm dialog. Cleared on open and
    /// every time the Confirm dialog re-opens.
    master_password: String,
    /// Disables the Confirm dialog's Continue button while the SDK
    /// validation call is in flight.
    validating: bool,
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    Close,
    /// Continue on the Compose dialog → open the master-password gate.
    Continue,
    FormatSelected(ExportFormatChoice),
    FilePasswordChanged(String),
    MasterPasswordChanged(String),
    /// Continue on the Confirm dialog → kick off SDK validation.
    ConfirmSubmit,
    /// Cancel / X / backdrop on the Confirm dialog → close it but keep the
    /// Compose dialog open so the user can adjust and retry.
    BackToCompose,
    /// SDK validation result. `Ok(uid)` proceeds to the save-as picker via
    /// [`ExportEvent::PickPathThenRun`] — the view rebuilds the SDK format
    /// from `self.selected_format` + `self.file_password` at that moment
    /// (the SDK's `ExportFormat` doesn't derive `Clone` / `Debug`, so we
    /// can't carry it inside an `ExportMessage`). The validated `uid` is
    /// echoed back so it flows through `PickPathThenRun` unchanged — an
    /// active-user switch in flight can't redirect the export to the
    /// wrong account. `Err` re-enables the Confirm button and surfaces
    /// an "Invalid master password" toast.
    ValidationCompleted(Result<UserId, String>),
    /// Result of the async SDK export — `Ok(Some(path))` is the file the
    /// App handler wrote the export to, `Ok(None)` means the user cancelled
    /// the native save dialog (silent close), `Err` is a human-readable
    /// error.
    Completed(Result<Option<String>, String>),
}

pub enum ExportEvent {
    /// Master password verified. App pops a native save dialog (rfd) for
    /// the user to pick a destination, then runs the SDK export with the
    /// already-folded format value. `uid` is captured at validation time
    /// so that an active-user switch between Confirm and the save dialog
    /// can't redirect the export to a different account's vault.
    PickPathThenRun {
        uid: UserId,
        format: bitwarden_exporters::ExportFormat,
    },
    /// Export finished — surface the saved-file path as a success toast.
    ToastSuccess(String),
    /// Export failed — surface the error as an error toast.
    ToastError(String),
    /// Confirm-step validation rejected the master password. App surfaces
    /// "Invalid master password" as an error toast; the Confirm dialog
    /// stays open so the user can retry.
    ToastInvalidMasterPassword,
}

impl ViewTypes for ExportView {
    type Message = ExportMessage;
    type Event = ExportEvent;
}

// ── Lifecycle ─────────────────────────────────────────────────────────────

impl ExportView {
    pub fn new() -> Self {
        Self {
            fade: FadeInOut::default(),
            confirm_fade: FadeInOut::default(),
            email: String::new(),
            selected_format: ExportFormatChoice::Json,
            file_password: String::new(),
            master_password: String::new(),
            validating: false,
        }
    }

    pub fn open(&mut self, email: String) {
        self.fade.open();
        self.confirm_fade.close();
        self.email = email;
        self.selected_format = ExportFormatChoice::Json;
        self.file_password.clear();
        self.master_password.clear();
        self.validating = false;
    }

    fn close_all(&mut self) {
        self.fade.close();
        self.confirm_fade.close();
        self.master_password.clear();
        self.validating = false;
    }

    pub fn update(&mut self, msg: ExportMessage, ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            ExportMessage::Close => {
                self.close_all();
                Outcome::None
            }
            ExportMessage::FormatSelected(f) => {
                self.selected_format = f;
                if !matches!(f, ExportFormatChoice::EncryptedJson) {
                    self.file_password.clear();
                }
                Outcome::None
            }
            ExportMessage::FilePasswordChanged(p) => {
                self.file_password = p;
                Outcome::None
            }
            ExportMessage::Continue => {
                self.master_password.clear();
                self.validating = false;
                self.confirm_fade.open();
                Outcome::task(fade_in_out::focus_after_open(CONFIRM_PASSWORD_FIELD_ID))
            }
            ExportMessage::MasterPasswordChanged(p) => {
                self.master_password = p;
                Outcome::None
            }
            ExportMessage::BackToCompose => {
                self.confirm_fade.close();
                self.master_password.clear();
                self.validating = false;
                Outcome::None
            }
            ExportMessage::ConfirmSubmit => {
                if self.master_password.is_empty() || self.validating {
                    return Outcome::None;
                }
                let Some(uid) = ctx.active_user.copied() else {
                    return Outcome::None;
                };
                self.validating = true;
                let password = self.master_password.clone();
                let mgr: Arc<ClientManager> = Arc::clone(ctx.client_manager);
                Outcome::spawn(
                    async move {
                        mgr.validate_master_password(&uid, password)
                            .await
                            .map(|()| uid)
                    },
                    ExportMessage::ValidationCompleted,
                )
            }
            ExportMessage::ValidationCompleted(Ok(uid)) => {
                self.validating = false;
                self.master_password.clear();
                // The validation task can outlive a user-initiated close
                // (outer X / backdrop). Drop the result on the floor in
                // that case rather than popping a save dialog from
                // nowhere.
                if !self.fade.is_open() {
                    return Outcome::None;
                }
                let format = self.selected_format.to_sdk(self.file_password.clone());
                Outcome::event(ExportEvent::PickPathThenRun { uid, format })
            }
            ExportMessage::ValidationCompleted(Err(_)) => {
                self.validating = false;
                Outcome::event(ExportEvent::ToastInvalidMasterPassword)
            }
            ExportMessage::Completed(Ok(Some(path))) => {
                self.file_password.clear();
                self.close_all();
                Outcome::event(ExportEvent::ToastSuccess(path))
            }
            ExportMessage::Completed(Ok(None)) => {
                // User cancelled the native save dialog — close the modals
                // silently with no toast.
                self.close_all();
                Outcome::None
            }
            ExportMessage::Completed(Err(err)) => {
                // Leave the modals open so the user can correct & retry —
                // typical case is a wrong / missing encryption password.
                self.validating = false;
                Outcome::event(ExportEvent::ToastError(err))
            }
        }
    }

    pub fn modal_view<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Option<Element<'a, ExportMessage, AppTheme>> {
        let progress = self.fade.progress_if_visible()?;
        let compose = self.compose_dialog(colors, progress);

        let Some(confirm_progress) = self.confirm_fade.progress_if_visible() else {
            return Some(compose);
        };
        let confirm = self.confirm_dialog(colors, confirm_progress);
        Some(stack![compose, confirm].into())
    }

    fn compose_dialog<'a>(
        &'a self,
        colors: &'a AppColors,
        progress: f32,
    ) -> Element<'a, ExportMessage, AppTheme> {
        let header = row![
            text(fl!("export-modal-title"))
                .size(20)
                .font(crate::APP_FONT_BOLD)
                .color(colors.text_primary),
            Space::new().width(Fill),
            buttons::ghost_icon(
                icons::X_LG.render(16.0, colors.text_primary),
                colors.item_hover,
            )
            .padding([6, 6])
            .on_press(ExportMessage::Close),
        ]
        .align_y(Alignment::Center);

        let banner_text = fl!("export-modal-banner", email = self.email.as_str());
        let banner = container(
            row![
                icons::INFO_CIRCLE.render(16.0, colors.accent),
                text(banner_text).size(13).color(colors.text_primary),
            ]
            .spacing(10)
            .align_y(Alignment::Start),
        )
        .padding(12)
        .width(Fill)
        .style(|theme: &AppTheme| {
            iced::widget::container::Style::default()
                .background(theme.colors.accent.scale_alpha(0.12))
                .border(iced::Border::default().rounded(RADIUS_LG))
        });

        // The picker sits directly on the dialog body (which is painted
        // with `card_bg`), so the floating label chip needs to match —
        // otherwise it shows as a contrasting tile on the border.
        let format_picker = inputs::select_field_on(
            fl!("export-modal-file-format-label"),
            Some(self.selected_format),
            ExportFormatChoice::ALL.to_vec(),
            |f: &ExportFormatChoice| f.label().to_string(),
            ExportMessage::FormatSelected,
            |c| c.card_bg,
            colors,
        );

        let mut body = column![header, banner, format_picker].spacing(16);

        if matches!(self.selected_format, ExportFormatChoice::EncryptedJson) {
            // Same chip-bg story as the format picker — sits directly on
            // the dialog `card_bg`. `bare_text_input` gives the standard
            // field look; `.secure(true)` masks the value.
            let password_input = inputs::bare_text_input(&self.file_password)
                .secure(true)
                .on_input(ExportMessage::FilePasswordChanged);
            body = body.push(inputs::field_frame_on(
                fl!("export-modal-password-label"),
                password_input,
                |c| c.card_bg,
                colors,
            ));
        }

        let continue_disabled = matches!(self.selected_format, ExportFormatChoice::EncryptedJson)
            && self.file_password.is_empty();

        let mut continue_btn =
            buttons::primary(text(fl!("export-modal-continue")).size(14)).padding([8, 20]);
        if !continue_disabled {
            continue_btn = continue_btn.on_press(ExportMessage::Continue);
        }
        let cancel_btn = buttons::secondary(text(fl!("export-modal-cancel")).size(14))
            .on_press(ExportMessage::Close)
            .padding([8, 20]);

        let body = body
            .push(
                row![continue_btn, cancel_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::from([20, 24]))
            .width(Fill);

        modal::dialog(
            460.0,
            None,
            |c| c.card_bg,
            progress,
            container(body),
            ExportMessage::Close,
        )
    }

    fn confirm_dialog<'a>(
        &'a self,
        colors: &'a AppColors,
        progress: f32,
    ) -> Element<'a, ExportMessage, AppTheme> {
        let header = row![
            text(fl!("export-confirm-title"))
                .size(20)
                .font(crate::APP_FONT_BOLD)
                .color(colors.text_primary),
            Space::new().width(Fill),
            buttons::ghost_icon(
                icons::X_LG.render(16.0, colors.text_primary),
                colors.item_hover,
            )
            .padding([6, 6])
            .on_press(ExportMessage::BackToCompose),
        ]
        .align_y(Alignment::Center);

        // `fl!` is a proc macro that requires a literal id, so the
        // format → key mapping has to live in match arms here. Mirrors
        // `verifyUser()` in the official client
        // (`clients/libs/tools/export/vault-export/vault-export-ui/src/components/export.component.ts`).
        // We don't expose an account-encrypted export today, so the
        // EncryptedJson variant always reads as file-encrypted.
        let warning_text = match self.selected_format {
            ExportFormatChoice::Json | ExportFormatChoice::Csv => {
                fl!("export-confirm-warning-unencrypted")
            }
            ExportFormatChoice::EncryptedJson => fl!("export-confirm-warning-file-encrypted"),
        };
        let warning = text(warning_text).size(14).color(colors.text_primary);

        let mut password_input = inputs::bare_text_input(&self.master_password)
            .id(CONFIRM_PASSWORD_FIELD_ID)
            .secure(true)
            .on_input(ExportMessage::MasterPasswordChanged);
        if !self.master_password.is_empty() && !self.validating {
            password_input = password_input.on_submit(ExportMessage::ConfirmSubmit);
        }
        let password_field = inputs::field_frame_on(
            fl!("export-confirm-password-label"),
            password_input,
            |c| c.card_bg,
            colors,
        );

        let helper = text(fl!("export-confirm-helper"))
            .size(12)
            .color(colors.text_secondary);

        let confirm_disabled = self.master_password.is_empty() || self.validating;
        let mut confirm_btn =
            buttons::primary(text(fl!("export-modal-continue")).size(14)).padding([8, 20]);
        if !confirm_disabled {
            confirm_btn = confirm_btn.on_press(ExportMessage::ConfirmSubmit);
        }
        let cancel_btn = buttons::secondary(text(fl!("export-modal-cancel")).size(14))
            .on_press(ExportMessage::BackToCompose)
            .padding([8, 20]);

        let body = column![
            header,
            warning,
            password_field,
            helper,
            row![confirm_btn, cancel_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(16)
        .padding(Padding::from([20, 24]))
        .width(Fill);

        modal::dialog(
            460.0,
            None,
            |c| c.card_bg,
            progress,
            container(body),
            ExportMessage::BackToCompose,
        )
    }
}


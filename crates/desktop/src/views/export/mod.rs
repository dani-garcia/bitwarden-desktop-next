//! Export vault modal — wired to the sidebar Export button.
//!
//! Format options mirror [`bitwarden_exporters::ExportFormat`] (Csv, Json,
//! EncryptedJson). Selecting `EncryptedJson` reveals a password input below
//! the picker; Submit hands the chosen format off to App, which calls the
//! SDK's `client.exporters().export_vault(...)` and writes the result to
//! disk.

use iced::{
    Alignment, Element, Fill, Padding,
    widget::{Space, column, container, row, text},
};

use crate::{
    app::{Outcome, UpdateCtx, ViewTypes},
    components::{FadeInOut, buttons, icons, inputs, modal},
    fl,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

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
}

impl std::fmt::Display for ExportFormatChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

// ── State ─────────────────────────────────────────────────────────────────

pub struct ExportView {
    pub fade: FadeInOut,
    /// Active user email — shown verbatim in the "Only items associated with"
    /// banner. Set via [`Self::open`] each time the modal is opened.
    email: String,
    selected_format: ExportFormatChoice,
    /// Password for the encrypted-json variant. Cleared on open and on
    /// successful submit. Only read when `selected_format == EncryptedJson`.
    password: String,
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    Close,
    Submit,
    FormatSelected(ExportFormatChoice),
    PasswordChanged(String),
    /// Result of the async SDK export — `Ok(path)` is the file the App
    /// handler wrote the export to, `Err` is a human-readable error.
    Completed(Result<String, String>),
}

pub enum ExportEvent {
    /// Run the export with the given SDK format. The view has already
    /// folded its picker selection + password field into a fully-shaped
    /// [`bitwarden_exporters::ExportFormat`] — App spawns the SDK call.
    Run(bitwarden_exporters::ExportFormat),
    /// Export finished — surface the saved-file path as a success toast.
    ToastSuccess(String),
    /// Export failed — surface the error as an error toast.
    ToastError(String),
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
            email: String::new(),
            selected_format: ExportFormatChoice::Json,
            password: String::new(),
        }
    }

    pub fn open(&mut self, email: String) {
        self.fade.open();
        self.email = email;
        self.selected_format = ExportFormatChoice::Json;
        self.password.clear();
    }

    pub fn update(&mut self, msg: ExportMessage, _ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            ExportMessage::Close => {
                self.fade.close();
                Outcome::None
            }
            ExportMessage::Submit => {
                let format = self.selected_format.to_sdk(self.password.clone());
                Outcome::event(ExportEvent::Run(format))
            }
            ExportMessage::FormatSelected(f) => {
                self.selected_format = f;
                if !matches!(f, ExportFormatChoice::EncryptedJson) {
                    self.password.clear();
                }
                Outcome::None
            }
            ExportMessage::PasswordChanged(p) => {
                self.password = p;
                Outcome::None
            }
            ExportMessage::Completed(Ok(path)) => {
                self.password.clear();
                self.fade.close();
                Outcome::event(ExportEvent::ToastSuccess(path))
            }
            ExportMessage::Completed(Err(err)) => {
                // Leave the modal open so the user can correct & retry —
                // typical case is a wrong / missing encryption password.
                Outcome::event(ExportEvent::ToastError(err))
            }
        }
    }

    pub fn modal_view<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Option<Element<'a, ExportMessage, AppTheme>> {
        let progress = self.fade.progress_if_visible()?;

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
            let password_input = inputs::bare_text_input(&self.password)
                .secure(true)
                .on_input(ExportMessage::PasswordChanged);
            body = body.push(inputs::field_frame_on(
                fl!("export-modal-password-label"),
                password_input.into(),
                |c| c.card_bg,
                colors,
            ));
        }

        let submit_disabled = matches!(self.selected_format, ExportFormatChoice::EncryptedJson)
            && self.password.is_empty();

        let mut submit_btn = buttons::primary(text(fl!("export-modal-submit")).size(14))
            .padding([8, 20]);
        if !submit_disabled {
            submit_btn = submit_btn.on_press(ExportMessage::Submit);
        }
        let cancel_btn = buttons::secondary(text(fl!("export-modal-cancel")).size(14))
            .on_press(ExportMessage::Close)
            .padding([8, 20]);

        let body = body
            .push(
                row![submit_btn, cancel_btn]
                    .spacing(8)
                    .align_y(Alignment::Center),
            )
            .padding(Padding::from([20, 24]))
            .width(Fill);

        Some(modal::dialog(
            460.0,
            None,
            |c| c.card_bg,
            progress,
            container(body).into(),
            ExportMessage::Close,
        ))
    }

}

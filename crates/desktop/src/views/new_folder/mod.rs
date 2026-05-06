//! New folder modal — wired to File → New folder.
//!
//! Single-field dialog: a `name` text input plus Save / Cancel. On Save
//! `ClientManager::create_folder` encrypts the view and writes it into the
//! per-user folder repo (no API call — same shape as `save_cipher`).
//!
//! Nested folders are a server-side convention: a name like `"Social/Forums"`
//! is stored as a single folder; the helper text under the field documents
//! it so users don't try to create the parent first.

mod handler;

use iced::{
    Element, Fill, Padding,
    widget::{self, column, container, text},
};

use crate::{
    app::{Outcome, UpdateCtx, ViewTypes},
    components::{FadeInOut, inputs, modal, toast::Toast},
    fl,
    theme::AppTheme,
};

pub const NAME_FIELD_ID: widget::Id = widget::Id::new("new-folder-name-field");

// ── State ─────────────────────────────────────────────────────────────────

pub struct NewFolderView {
    pub(super) fade: FadeInOut,
    name: String,
    /// True while the SDK encrypt + repo write is in flight. Disables the
    /// Save button + name input so submit-spamming can't double-create.
    saving: bool,
}

#[derive(Debug, Clone)]
pub enum NewFolderMessage {
    Close,
    NameChanged(String),
    Submit,
    /// SDK round-trip finished — `Ok(())` on success (the modal doesn't need
    /// the saved view back), `Err` is a human-readable message.
    Saved(Result<(), String>),
}

pub enum NewFolderEvent {
    /// Encrypt + persist the entered name. App spawns the SDK call.
    Run(String),
}

impl ViewTypes for NewFolderView {
    type Message = NewFolderMessage;
    type Event = NewFolderEvent;
}

// ── Lifecycle ─────────────────────────────────────────────────────────────

impl NewFolderView {
    pub fn new() -> Self {
        Self {
            fade: FadeInOut::default(),
            name: String::new(),
            saving: false,
        }
    }

    pub fn open(&mut self) {
        self.fade.open();
        self.name.clear();
        self.saving = false;
    }

    pub fn update(&mut self, msg: NewFolderMessage, _ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            NewFolderMessage::Close => {
                self.fade.close();
                Outcome::None
            }
            NewFolderMessage::NameChanged(n) => {
                if !self.saving {
                    self.name = n;
                }
                Outcome::None
            }
            NewFolderMessage::Submit => {
                let trimmed = self.name.trim();
                if trimmed.is_empty() || self.saving {
                    return Outcome::None;
                }
                self.saving = true;
                Outcome::event(NewFolderEvent::Run(trimmed.to_string()))
            }
            NewFolderMessage::Saved(Ok(_)) => {
                self.saving = false;
                self.fade.close();
                Outcome::toast(Toast::success(fl!("new-folder-toast-success"), None))
            }
            NewFolderMessage::Saved(Err(err)) => {
                self.saving = false;
                tracing::warn!(%err, "create folder failed");
                Outcome::toast(Toast::error(
                    fl!("new-folder-toast-failed-body"),
                    Some(&fl!("new-folder-toast-failed-title")),
                ))
            }
        }
    }

    pub fn modal_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, NewFolderMessage, AppTheme>> {
        let progress = self.fade.progress_if_visible()?;

        let header = modal::dialog_header(
            fl!("new-folder-modal-title"),
            NewFolderMessage::Close,
            ctx.colors,
        );

        // Modal sits on `card_bg`, so the floating-label chip needs the
        // matching surface colour to blend cleanly.
        let mut name_field = inputs::text_field(
            fl!("new-folder-modal-field-label"),
            self.name.as_str(),
            ctx.colors,
        )
        .id(NAME_FIELD_ID.clone())
        .chip_bg(|c| c.card_bg)
        .disabled(self.saving);
        if !self.saving {
            name_field = name_field
                .on_input(NewFolderMessage::NameChanged)
                .on_submit(NewFolderMessage::Submit);
        }

        let helper = text(fl!("new-folder-modal-helper"))
            .size(12)
            .color(ctx.colors.text_secondary);

        let submit_disabled = self.saving || self.name.trim().is_empty();
        let on_submit = (!submit_disabled).then_some(NewFolderMessage::Submit);
        let footer = modal::footer_actions(
            fl!("new-folder-modal-save"),
            on_submit,
            fl!("new-folder-modal-cancel"),
            NewFolderMessage::Close,
        );

        let body = column![header, name_field, helper, footer]
            .spacing(14)
            .padding(Padding::from([20, 24]))
            .width(Fill);

        Some(modal::dialog(
            480.0,
            None,
            |c| c.card_bg,
            progress,
            container(body),
            NewFolderMessage::Close,
        ))
    }
}

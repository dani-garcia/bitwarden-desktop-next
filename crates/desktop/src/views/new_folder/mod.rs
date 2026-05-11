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

    #[cfg(test)]
    fn name_for_test(&self) -> &str {
        &self.name
    }

    #[cfg(test)]
    fn is_saving_for_test(&self) -> bool {
        self.saving
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

#[cfg(test)]
mod tests_snapshot {
    use super::*;
    use crate::{test_support, test_support::TestRenderCtx};

    #[tokio::test(flavor = "current_thread")]
    async fn new_folder_modal() {
        test_support::init();
        let mut view = NewFolderView::new();
        view.open();
        test_support::settle_animations();

        let mut render = TestRenderCtx::default();
        for (theme, suffix) in [
            (AppTheme::light(), "light"),
            (AppTheme::dark(), "dark"),
        ] {
            render.colors = theme.colors;
            let element = view
                .modal_view(&render.as_ctx())
                .expect("modal renders while open");
            test_support::assert_snapshot(
                format!("tests/snapshots/new_folder_modal_{suffix}"),
                &theme,
                element,
            );
        }
    }
}

#[cfg(test)]
mod tests_interaction {
    use super::*;
    use crate::test_support::{self, TestRenderCtx, TestUpdateCtx};

    fn run(view: &mut NewFolderView, msg: NewFolderMessage) -> Outcome<NewFolderView> {
        let mut owned = TestUpdateCtx::default();
        view.update(msg, owned.as_ctx())
    }

    #[tokio::test(flavor = "current_thread")]
    async fn type_name_then_click_save_submits() {
        test_support::init();
        let mut view = NewFolderView::new();
        view.open();
        test_support::settle_animations();

        let render = TestRenderCtx::default();

        // Focus the name field via its stable widget::Id, then type.
        let element = view
            .modal_view(&render.as_ctx())
            .expect("modal renders while open");
        let messages = test_support::drive_element(element, |ui| {
            ui.click(NAME_FIELD_ID.clone())
                .expect("name field has a click target");
            ui.typewrite("Social");
        });

        // text_input emits one NameChanged per keystroke with the cumulative
        // text. Apply them all so the next render reflects the full typed
        // value.
        for msg in messages {
            let _ = run(&mut view, msg);
        }
        assert_eq!(view.name_for_test(), "Social");
        assert!(!view.is_saving_for_test());

        // With a non-empty name the Save button is now enabled. Click it.
        let element = view
            .modal_view(&render.as_ctx())
            .expect("modal renders while open");
        let messages = test_support::drive_element(element, |ui| {
            ui.click("Save").expect("Save button enabled");
        });
        let saw_submit = messages
            .iter()
            .any(|m| matches!(m, NewFolderMessage::Submit));
        assert!(saw_submit, "expected Submit in {messages:?}");

        for msg in messages {
            let _ = run(&mut view, msg);
        }
        assert!(view.is_saving_for_test(), "Submit flipped saving=true");
    }
}

#[cfg(test)]
mod tests_update {
    use super::*;
    use crate::{
        components::toast::ToastStatus,
        test_support::{OutcomeExt, TestUpdateCtx},
    };

    fn run(view: &mut NewFolderView, msg: NewFolderMessage) -> Outcome<NewFolderView> {
        let mut owned = TestUpdateCtx::default();
        view.update(msg, owned.as_ctx())
    }

    #[test]
    fn name_changed_updates_when_not_saving() {
        let mut view = NewFolderView::new();
        run(&mut view, NewFolderMessage::NameChanged("Social".into())).expect_none();
        assert_eq!(view.name_for_test(), "Social");
    }

    #[test]
    fn name_changed_is_ignored_while_saving() {
        let mut view = NewFolderView::new();
        // Get into the saving state first.
        view.open();
        run(&mut view, NewFolderMessage::NameChanged("First".into())).expect_none();
        let _ = run(&mut view, NewFolderMessage::Submit);
        assert!(view.is_saving_for_test());

        run(&mut view, NewFolderMessage::NameChanged("Second".into())).expect_none();
        assert_eq!(
            view.name_for_test(),
            "First",
            "name field is locked while the SDK call is in flight"
        );
    }

    #[test]
    fn submit_with_empty_name_returns_none() {
        let mut view = NewFolderView::new();
        run(&mut view, NewFolderMessage::Submit).expect_none();
        assert!(!view.is_saving_for_test());
    }

    #[test]
    fn submit_with_whitespace_only_name_returns_none() {
        let mut view = NewFolderView::new();
        let _ = run(&mut view, NewFolderMessage::NameChanged("   \t".into()));
        run(&mut view, NewFolderMessage::Submit).expect_none();
        assert!(!view.is_saving_for_test());
    }

    #[test]
    fn submit_with_valid_name_emits_run_event_and_flips_saving() {
        let mut view = NewFolderView::new();
        let _ = run(&mut view, NewFolderMessage::NameChanged("  Social  ".into()));
        let NewFolderEvent::Run(name) = run(&mut view, NewFolderMessage::Submit).expect_event();
        assert_eq!(name, "Social");
        assert!(view.is_saving_for_test());
    }

    #[test]
    fn submit_while_already_saving_returns_none() {
        let mut view = NewFolderView::new();
        let _ = run(&mut view, NewFolderMessage::NameChanged("Social".into()));
        let _ = run(&mut view, NewFolderMessage::Submit);
        assert!(view.is_saving_for_test());

        run(&mut view, NewFolderMessage::Submit).expect_none();
        assert!(view.is_saving_for_test(), "still saving");
    }

    #[test]
    fn saved_ok_clears_saving_and_emits_success_toast() {
        let mut view = NewFolderView::new();
        let _ = run(&mut view, NewFolderMessage::NameChanged("Social".into()));
        let _ = run(&mut view, NewFolderMessage::Submit);

        let toast = run(&mut view, NewFolderMessage::Saved(Ok(()))).expect_toast();
        assert!(matches!(toast.status, ToastStatus::Success));
        assert!(!view.is_saving_for_test());
    }

    #[test]
    fn saved_err_clears_saving_and_emits_error_toast() {
        let mut view = NewFolderView::new();
        let _ = run(&mut view, NewFolderMessage::NameChanged("Social".into()));
        let _ = run(&mut view, NewFolderMessage::Submit);

        let toast = run(&mut view, NewFolderMessage::Saved(Err("boom".into()))).expect_toast();
        assert!(matches!(toast.status, ToastStatus::Error));
        assert!(!view.is_saving_for_test());
    }
}

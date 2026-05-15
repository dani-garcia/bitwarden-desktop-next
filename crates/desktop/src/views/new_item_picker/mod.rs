//! New-item picker modal — fired from the vault header `+ New` button.
//!
//! Centered modal with a "Choose item to add" title and a two-column grid
//! of picker tiles covering Login / Card / Bank account / Identity / Note /
//! SSH key / Folder. Picking a cipher type closes the modal and routes
//! through the same [`MenuAction::NewItem`] path the File menu uses; picking
//! Folder closes the modal and opens the existing [new_folder] dialog. The
//! view itself owns no SDK state — App handles the side effects.
//!
//! Lives at top level (not as a vault sub-widget) so the Folder tile can
//! launch the new-folder modal without vault needing to know about it.
//!
//! [new_folder]: crate::views::new_folder

mod handler;

use bitwarden_vault::CipherType;
use iced::{
    Element, Fill, Length,
    widget::{Space, column, container, row},
};

use crate::{
    app::{Outcome, RenderCtx, UpdateCtx, View},
    components::{FadeInOut, icons, modal, picker_tile},
    fl,
    theme::AppTheme,
};

// ── State ─────────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct NewItemPickerView {
    fade: FadeInOut,
}

#[derive(Debug, Clone, Copy)]
pub enum NewItemPickerMessage {
    Close,
    /// User picked a cipher type tile.
    PickType(CipherType),
    /// User picked the Folder tile.
    PickFolder,
}

pub enum NewItemPickerEvent {
    /// Route a cipher-type pick through the same path as the File menu's
    /// `New item` submenu. App fires `MenuAction::NewItem(t)`.
    NewItem(CipherType),
    /// Open the existing new-folder modal. App fires `MenuAction::NewFolder`.
    NewFolder,
}

// ── Lifecycle ─────────────────────────────────────────────────────────────

impl NewItemPickerView {
    pub fn open(&mut self) {
        self.fade.open();
    }
}

impl View for NewItemPickerView {
    type Message = NewItemPickerMessage;
    type Event = NewItemPickerEvent;

    fn update(&mut self, msg: NewItemPickerMessage, _ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            NewItemPickerMessage::Close => {
                self.fade.close();
                Outcome::None
            }
            NewItemPickerMessage::PickType(t) => {
                self.fade.close();
                Outcome::event(NewItemPickerEvent::NewItem(t))
            }
            NewItemPickerMessage::PickFolder => {
                self.fade.close();
                Outcome::event(NewItemPickerEvent::NewFolder)
            }
        }
    }

    fn should_render(&self) -> bool {
        self.fade.is_visible()
    }

    fn view<'a>(&'a self, ctx: &RenderCtx<'a>) -> Element<'a, NewItemPickerMessage, AppTheme> {
        let progress = self.fade.progress_when_visible();

        let header =
            modal::dialog_header(fl!("picker-title"), NewItemPickerMessage::Close, ctx.colors);

        // Tile rows: two tiles per row except the trailing Folder row which
        // sits alone in the left column (the design leaves the right column
        // empty rather than centering Folder).
        let tiles = [
            (
                icons::BWI_LOGIN,
                fl!("menu-file-new-item-login"),
                fl!("picker-login-subtitle"),
                NewItemPickerMessage::PickType(CipherType::Login),
            ),
            (
                icons::BWI_CREDIT_CARD,
                fl!("menu-file-new-item-card"),
                fl!("picker-card-subtitle"),
                NewItemPickerMessage::PickType(CipherType::Card),
            ),
            (
                icons::BWI_BANK,
                fl!("menu-file-new-item-bank-account"),
                fl!("picker-bank-account-subtitle"),
                NewItemPickerMessage::PickType(CipherType::BankAccount),
            ),
            (
                icons::BWI_IDENTITY,
                fl!("menu-file-new-item-identity"),
                fl!("picker-identity-subtitle"),
                NewItemPickerMessage::PickType(CipherType::Identity),
            ),
            (
                icons::BWI_NOTE,
                fl!("menu-file-new-item-secure-note"),
                fl!("picker-secure-note-subtitle"),
                NewItemPickerMessage::PickType(CipherType::SecureNote),
            ),
            (
                icons::BWI_KEY,
                fl!("menu-file-new-item-ssh-key"),
                fl!("picker-ssh-key-subtitle"),
                NewItemPickerMessage::PickType(CipherType::SshKey),
            ),
            (
                icons::BWI_DRIVERS_LICENSE,
                fl!("menu-file-new-item-drivers-license"),
                fl!("picker-drivers-license-subtitle"),
                NewItemPickerMessage::PickType(CipherType::DriversLicense),
            ),
            (
                icons::BWI_PASSPORT,
                fl!("menu-file-new-item-passport"),
                fl!("picker-passport-subtitle"),
                NewItemPickerMessage::PickType(CipherType::Passport),
            ),
            (
                icons::BWI_FOLDER,
                fl!("picker-folder"),
                fl!("picker-folder-subtitle"),
                NewItemPickerMessage::PickFolder,
            ),
        ];

        let mut grid = column![].spacing(8);
        let mut iter = tiles.into_iter();
        while let Some((icon, title, subtitle, msg)) = iter.next() {
            let left = picker_tile::view(icon, title, subtitle, msg, ctx.colors);
            let right: Element<'_, NewItemPickerMessage, AppTheme> =
                if let Some((ricon, rtitle, rsubtitle, rmsg)) = iter.next() {
                    picker_tile::view(ricon, rtitle, rsubtitle, rmsg, ctx.colors)
                } else {
                    Space::new().width(Length::Fill).into()
                };
            grid = grid.push(row![left, right].spacing(8));
        }

        let body = column![header, grid]
            .spacing(14)
            .padding(modal::BODY_PADDING)
            .width(Fill);

        modal::dialog(
            720.0,
            None,
            |c| c.card_bg,
            progress,
            container(body),
            NewItemPickerMessage::Close,
        )
    }
}

#[cfg(test)]
mod tests_update {
    use super::*;
    use crate::test_support::{OutcomeExt, ViewTestExt};

    #[tokio::test(flavor = "current_thread")]
    async fn close_closes_fade() {
        let mut view = NewItemPickerView::default();
        view.open();
        view.run(NewItemPickerMessage::Close).await.expect_none();
        assert!(!view.fade.is_open());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pick_type_closes_fade_and_emits_event() {
        let mut view = NewItemPickerView::default();
        view.open();
        let ev = view
            .run(NewItemPickerMessage::PickType(CipherType::Login))
            .await
            .expect_event();
        assert!(matches!(ev, NewItemPickerEvent::NewItem(CipherType::Login)));
        assert!(!view.fade.is_open());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn pick_folder_closes_fade_and_emits_event() {
        let mut view = NewItemPickerView::default();
        view.open();
        let ev = view
            .run(NewItemPickerMessage::PickFolder)
            .await
            .expect_event();
        assert!(matches!(ev, NewItemPickerEvent::NewFolder));
        assert!(!view.fade.is_open());
    }
}

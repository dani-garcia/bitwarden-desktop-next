//! `VaultView::view` plus `sheet_view` / `modal_view` / view-internal builders.

use std::sync::Arc;

use bitwarden_vault::{CipherListView, CipherType};
use iced::{
    Alignment, Border, Color, Element, Length,
    widget::{column, container, row, text},
};

use crate::{
    app::{Overlay, RenderCtx},
    components::{
        account_switcher,
        bottom_sheet::{self, SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX},
        buttons, drop_down,
        icons::{self, BwiIcon},
        list_pane,
    },
    fl,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

use super::{
    VaultMessage,
    state::VaultView,
    widgets::{
        cipher_detail::{self, CipherDetailMessage},
        cipher_edit, item_list, search_bar,
    },
};

impl VaultView {
    pub(super) fn render<'a>(&'a self, ctx: &RenderCtx<'a>) -> Element<'a, VaultMessage, AppTheme> {
        let active_user = ctx.active_user.expect("Screen::Vault without active_user");
        let user_cache = self.items.get(active_user);
        let cached_items: &[Arc<CipherListView>] =
            user_cache.map(|ic| ic.cached.as_slice()).unwrap_or(&[]);

        // Below `SHEET_BREAKPOINT_PX` the right pane is composed at the App
        // level as a bottom sheet (`render_sheet`), so build it here only in
        // wide mode.
        let right_pane = (ctx.window_width >= SHEET_BREAKPOINT_PX
            && (self.selection.detail.is_some() || self.selection.form.is_some()))
        .then(|| self.detail_or_form_pane(ctx.colors, 0.0));

        list_pane::render(
            &self.pane,
            self.list_content(ctx, cached_items),
            right_pane,
            VaultMessage::PaneResized,
            ctx.window_width,
        )
    }

    /// In narrow mode (`window_width < SHEET_BREAKPOINT_PX`) with a detail
    /// selected, returns the bottom-sheet element that the app composes on
    /// top of the entire window (including sidebar and title bar). Returns
    /// `None` otherwise.
    pub(super) fn render_sheet<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        if ctx.window_width >= SHEET_BREAKPOINT_PX {
            return None;
        }
        if self.selection.detail.is_none() && self.selection.form.is_none() {
            return None;
        }
        let progress = self.selection.sheet_fade.progress_if_visible()?;
        let pane = self.detail_or_form_pane(ctx.colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            progress,
            Some(VaultMessage::CloseCipherDetail),
        ))
    }

    /// Returns the delete-confirmation modal when armed, `None` otherwise.
    /// Composed by the App on top of the vault view so the backdrop covers
    /// the sidebar and title bar.
    pub(super) fn render_overlay<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        let progress = self.selection.confirm_delete.progress_if_visible()?;
        let item_name = self
            .selection
            .detail
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("");

        Some(crate::components::modal::confirm_dialog(
            fl!("vault-delete-modal-title"),
            fl!("vault-delete-modal-body", name = item_name),
            fl!("vault-delete-modal-confirm"),
            fl!("vault-delete-modal-cancel"),
            VaultMessage::ConfirmDeleteSelected,
            VaultMessage::CancelDeleteSelected,
            ctx.colors,
            progress,
        ))
    }

    /// Builds the right-side pane content: either the editable `cipher_edit`
    /// when `selection.form.is_some()`, or the read-only `cipher_detail`.
    fn detail_or_form_pane<'a>(
        &'a self,
        colors: &'a AppColors,
        top_radius: f32,
    ) -> Element<'a, VaultMessage, AppTheme> {
        if let Some(form) = self.selection.form.as_ref() {
            cipher_edit::view(form, colors, top_radius).map(Into::into)
        } else {
            let item = self
                .selection
                .detail
                .as_ref()
                .expect("detail_or_form_pane called without a selection");
            cipher_detail::view(item, colors, top_radius).map(|msg| match msg {
                CipherDetailMessage::Close => VaultMessage::CloseCipherDetail,
                other => VaultMessage::CipherDetail(other),
            })
        }
    }

    /// Builds the list pane content (header + search + item list).
    fn list_content<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
        cached_items: &'a [Arc<CipherListView>],
    ) -> Element<'a, VaultMessage, AppTheme> {
        let active_email = ctx
            .active_email
            .expect("Screen::Vault without active_email");

        let new_button_trigger = list_pane::new_item_button(
            fl!("vault-new-button"),
            VaultMessage::ToggleNewItemMenu,
            ctx.colors,
        );
        let new_item_menu_open = ctx.open_overlay == Some(Overlay::NewItemMenu);
        let new_button = drop_down::DropDown::new(
            new_button_trigger,
            new_item_menu(ctx.colors),
            new_item_menu_open,
        )
        .on_dismiss(VaultMessage::ToggleNewItemMenu)
        .alignment(drop_down::Alignment::BelowRight)
        .width(220.0)
        .offset(4.0);

        let account_switcher_open = ctx.open_overlay == Some(Overlay::AccountSwitcher);
        let avatar = account_switcher::header_switcher(
            active_email,
            ctx.accounts,
            account_switcher_open,
            ctx.colors,
        )
        .map(Into::into);

        let search = search_bar::view(&self.search_query).map(Into::into);
        let item_list = item_list::view(ctx, cached_items, self.selection.item, self.list_scroll)
            .map(Into::into);

        list_pane::layout(
            fl!("vault-title"),
            new_button,
            avatar,
            search,
            item_list,
            ctx.colors,
        )
    }
}

/// Floating panel for the +New dropdown — five rows, one per cipher type.
/// Reuses the File-menu Fluent keys so the labels match the menu items that
/// fire the same `VaultMessage::NewItem(CipherType)`.
fn new_item_menu<'a>(colors: &'a AppColors) -> Element<'a, VaultMessage, AppTheme> {
    let rows = [
        (
            icons::BWI_LOGIN,
            fl!("menu-file-new-item-login"),
            CipherType::Login,
        ),
        (
            icons::BWI_CREDIT_CARD,
            fl!("menu-file-new-item-card"),
            CipherType::Card,
        ),
        (
            icons::BWI_IDENTITY,
            fl!("menu-file-new-item-identity"),
            CipherType::Identity,
        ),
        (
            icons::BWI_NOTE,
            fl!("menu-file-new-item-secure-note"),
            CipherType::SecureNote,
        ),
        (
            icons::BWI_KEY,
            fl!("menu-file-new-item-ssh-key"),
            CipherType::SshKey,
        ),
    ];

    let body = column(
        rows.into_iter()
            .map(|(icon, label, t)| new_item_row(icon, label, t, colors)),
    )
    .spacing(0);

    container(body)
        .width(220)
        .padding([8, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(RADIUS_LG),
                )
        })
        .into()
}

fn new_item_row<'a>(
    icon: BwiIcon,
    label: String,
    cipher_type: CipherType,
    colors: &'a AppColors,
) -> Element<'a, VaultMessage, AppTheme> {
    buttons::ghost(
        row![
            icon.render(16.0, colors.accent),
            text(label).size(14).color(colors.text_primary),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        false,
        Color::TRANSPARENT,
        colors.item_hover,
        0.0,
    )
    .on_press(VaultMessage::NewItem(cipher_type))
    .padding([8, 12])
    .width(Length::Fill)
    .into()
}

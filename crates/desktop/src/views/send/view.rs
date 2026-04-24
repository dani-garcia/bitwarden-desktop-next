//! `SendView::view` + `sheet_view` / `modal_view` — parallel to the vault
//! view's rendering pattern.

use std::sync::Arc;

use bitwarden_send::SendView as SdkSendView;
use iced::{
    Alignment, Border, Element, Fill, Length, Padding,
    widget::{Space, column, container, row, text},
};

use crate::{
    components::{
        account_switcher::{self, AccountSwitcherMessage},
        bottom_sheet, buttons, collapsible_pane, icons,
    },
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX, SendMessage,
    state::SendView,
    widgets::{send_form, send_list},
};

impl SendView {
    pub fn view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Element<'a, SendMessage, AppTheme> {
        let active_user = ctx.active_user.expect("Screen::Send without active_user");
        let cached_items: &[Arc<SdkSendView>] = self
            .items
            .get(active_user)
            .map(|ic| ic.cached.as_slice())
            .unwrap_or(&[]);

        // Wide: collapsible-pane with the form on the right (kept mounted
        // so iced's `pane_grid::diff` never drops its child state — see
        // `components::collapsible_pane` for the details).
        // Narrow: the form renders as a bottom sheet via `sheet_view`, so
        // the pane grid drops out entirely here.
        let content_area_inner: Element<'a, SendMessage, AppTheme> =
            if ctx.window_width >= SHEET_BREAKPOINT_PX {
                let right = self
                    .selection
                    .form
                    .as_ref()
                    .map(|_| self.form_pane(ctx.colors, 0.0));
                collapsible_pane::view(
                    &self.pane,
                    self.list_content(cached_items, ctx),
                    right,
                    SendMessage::PaneResized,
                )
            } else {
                self.list_content(cached_items, ctx)
            };

        container(content_area_inner)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.background)
                    .border(Border::default().rounded(iced::border::top_left(10)))
            })
            .into()
    }

    pub fn sheet_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, SendMessage, AppTheme>> {
        if ctx.window_width >= SHEET_BREAKPOINT_PX {
            return None;
        }
        self.selection.form.as_ref()?;
        let pane = self.form_pane(ctx.colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            Some(SendMessage::CloseFormPane),
        ))
    }

    pub fn modal_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, SendMessage, AppTheme>> {
        if !self.selection.confirm_delete {
            return None;
        }
        let colors = ctx.colors;
        let item_name = self
            .selection
            .form
            .as_ref()
            .map(|f| f.name().to_owned())
            .unwrap_or_default();

        let title = text(fl!("send-delete-modal-title"))
            .size(18)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);
        let body = text(fl!("send-delete-modal-body", name = item_name))
            .size(14)
            .color(colors.text_primary);

        let cancel_btn = buttons::secondary(text(fl!("send-delete-modal-cancel")).size(14))
            .on_press(SendMessage::CancelDeleteSelected)
            .padding([8, 20]);
        let confirm_btn = buttons::primary(text(fl!("send-delete-modal-confirm")).size(14))
            .on_press(SendMessage::ConfirmDeleteSelected)
            .padding([8, 20]);

        let dialog_inner: Element<'_, SendMessage, AppTheme> = column![
            title,
            body,
            row![Space::new().width(Fill), cancel_btn, confirm_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(Padding::from([16, 20]))
        .width(Length::Fixed(380.0))
        .into();

        let dialog = container(dialog_inner).style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(crate::theme::RADIUS_LG))
        });

        Some(crate::components::modal::view(
            dialog.into(),
            SendMessage::CancelDeleteSelected,
        ))
    }

    fn form_pane<'a>(
        &'a self,
        colors: &'a AppColors,
        top_radius: f32,
    ) -> Element<'a, SendMessage, AppTheme> {
        let form = self
            .selection
            .form
            .as_ref()
            .expect("form_pane called without a form");
        send_form::view(form, colors, top_radius).map(SendMessage::SendForm)
    }

    fn list_content<'a>(
        &'a self,
        cached_items: &'a [Arc<SdkSendView>],
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Element<'a, SendMessage, AppTheme> {
        let colors = ctx.colors;
        let active_email = ctx
            .active_email
            .expect("Screen::Send without active_email");

        let title = text(fl!("send-title"))
            .size(28)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);

        let new_button = buttons::primary(
            row![
                icons::PLUS.render(14.0, colors.card_bg),
                text(fl!("send-new-button")).size(14),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(SendMessage::NewItem)
        .padding(Padding {
            top: 8.0,
            right: 16.0,
            bottom: 8.0,
            left: 12.0,
        });

        let account_switcher_open = ctx.open_overlay == Some(crate::app::Overlay::AccountSwitcher);
        let avatar_trigger = account_switcher::avatar_trigger(active_email, colors)
            .map(SendMessage::AccountSwitcher);
        let dd_panel = account_switcher::dropdown(Some(active_email), ctx.accounts, colors)
            .map(SendMessage::AccountSwitcher);
        let avatar: Element<'a, SendMessage, AppTheme> =
            crate::components::drop_down::DropDown::new(
                avatar_trigger,
                dd_panel,
                account_switcher_open,
            )
            .on_dismiss(SendMessage::AccountSwitcher(
                AccountSwitcherMessage::ToggleDropdown,
            ))
            .alignment(crate::components::drop_down::Alignment::BelowRight)
            .width(360.0)
            .offset(4.0)
            .into();

        let content_header = container(
            row![title, Space::new().width(Fill), new_button, avatar]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding([16, 24])
        .width(Fill);

        let search = send_list::search_view(&self.search_query).map(SendMessage::Search);
        let search_row = container(search)
            .padding(Padding {
                top: 0.0,
                right: 24.0,
                bottom: 8.0,
                left: 24.0,
            })
            .width(Fill);

        let list_body: Element<'a, SendMessage, AppTheme> = if cached_items.is_empty() {
            send_list::empty_state(colors).map(SendMessage::ItemList)
        } else {
            send_list::view(cached_items, self.selection.item, self.list_scroll, colors)
                .map(SendMessage::ItemList)
        };

        column![content_header, search_row, list_body]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

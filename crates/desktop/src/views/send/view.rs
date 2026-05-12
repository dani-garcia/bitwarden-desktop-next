use std::sync::Arc;

use bitwarden_send::SendView as SdkSendView;
use iced::{
    Alignment, Border, Element, Fill, Padding,
    widget::{Space, column, container, row, text},
};

use crate::{
    app::{Overlay, RenderCtx},
    components::{
        account_switcher,
        bottom_sheet::{self, SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX},
        buttons, collapsible_pane, icons,
    },
    fl,
    theme::{AppColors, AppTheme},
};

use super::{
    SendMessage,
    state::SendView,
    widgets::{send_edit, send_list},
};

impl SendView {
    pub(super) fn render<'a>(&'a self, ctx: &RenderCtx<'a>) -> Element<'a, SendMessage, AppTheme> {
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
                    self.list_content(ctx, cached_items),
                    right,
                    SendMessage::PaneResized,
                )
            } else {
                self.list_content(ctx, cached_items)
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

    pub(super) fn render_sheet<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
    ) -> Option<Element<'a, SendMessage, AppTheme>> {
        if ctx.window_width >= SHEET_BREAKPOINT_PX {
            return None;
        }
        self.selection.form.as_ref()?;
        let progress = self.selection.sheet_fade.progress_if_visible()?;
        let pane = self.form_pane(ctx.colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            progress,
            Some(SendMessage::CloseFormPane),
        ))
    }

    pub(super) fn render_overlay<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
    ) -> Option<Element<'a, SendMessage, AppTheme>> {
        let progress = self.selection.confirm_delete.progress_if_visible()?;
        let colors = ctx.colors;
        let item_name = self
            .selection
            .form
            .as_ref()
            .map(|f| f.name().to_owned())
            .unwrap_or_default();

        Some(crate::components::modal::confirm_dialog(
            fl!("send-delete-modal-title"),
            fl!("send-delete-modal-body", name = item_name),
            fl!("send-delete-modal-confirm"),
            fl!("send-delete-modal-cancel"),
            SendMessage::ConfirmDeleteSelected,
            SendMessage::CancelDeleteSelected,
            colors,
            progress,
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
        send_edit::view(form, colors, top_radius).map(SendMessage::SendEdit)
    }

    fn list_content<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
        cached_items: &'a [Arc<SdkSendView>],
    ) -> Element<'a, SendMessage, AppTheme> {
        let colors = ctx.colors;
        let active_email = ctx.active_email.expect("Screen::Send without active_email");

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

        let account_switcher_open = ctx.open_overlay == Some(Overlay::AccountSwitcher);
        let avatar = account_switcher::header_switcher(
            active_email,
            ctx.accounts,
            account_switcher_open,
            colors,
        )
        .map(SendMessage::AccountSwitcher);

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

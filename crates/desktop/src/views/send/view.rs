use std::sync::Arc;

use bitwarden_send::SendView as SdkSendView;
use iced::Element;

use crate::{
    app::{Overlay, RenderCtx},
    components::{
        account_switcher,
        bottom_sheet::{self, SHEET_BREAKPOINT_PX, SHEET_TOP_INSET_PX, SHEET_TOP_RADIUS_PX},
        list_pane,
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

        // Below `SHEET_BREAKPOINT_PX` the form renders separately as a bottom
        // sheet (`render_sheet`), so build the right pane here only in wide
        // mode.
        let right_pane = (ctx.window_width >= SHEET_BREAKPOINT_PX)
            .then(|| {
                self.selection
                    .form
                    .as_ref()
                    .map(|_| self.form_pane(ctx.colors, 0.0))
            })
            .flatten();

        list_pane::render(
            &self.pane,
            self.list_content(ctx, cached_items),
            right_pane,
            SendMessage::PaneResized,
            ctx.window_width,
        )
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
            ctx.colors,
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
        send_edit::view(form, colors, top_radius).map(Into::into)
    }

    fn list_content<'a>(
        &'a self,
        ctx: &RenderCtx<'a>,
        cached_items: &'a [Arc<SdkSendView>],
    ) -> Element<'a, SendMessage, AppTheme> {
        let active_email = ctx.active_email.expect("Screen::Send without active_email");

        let new_button =
            list_pane::new_item_button(fl!("send-new-button"), SendMessage::NewItem, ctx.colors);

        let account_switcher_open = ctx.open_overlay == Some(Overlay::AccountSwitcher);
        let avatar = account_switcher::header_switcher(
            active_email,
            ctx.accounts,
            account_switcher_open,
            ctx.colors,
        )
        .map(Into::into);

        let search = send_list::search_view(&self.search_query).map(Into::into);
        let list_body: Element<'a, SendMessage, AppTheme> = if cached_items.is_empty() {
            send_list::empty_state(ctx.colors).map(Into::into)
        } else {
            send_list::view(
                cached_items,
                self.selection.item,
                self.list_scroll,
                ctx.colors,
            )
            .map(Into::into)
        };

        list_pane::layout(
            fl!("send-title"),
            new_button,
            avatar,
            search,
            list_body,
            ctx.colors,
        )
    }
}

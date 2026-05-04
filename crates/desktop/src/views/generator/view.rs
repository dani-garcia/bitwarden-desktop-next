//! `GeneratorView::modal_view` and the supporting render helpers (tab row,
//! value card, history disclosure, section heading).

use std::time::Instant;

use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding,
    widget::{Space, Stack, button, column, container, row, scrollable, text},
};

use crate::{
    components::{self, buttons, icons, modal},
    fl,
    theme::{AppColors, AppTheme, RADIUS_LG, RADIUS_PILL},
};

use super::{
    GeneratorView,
    history,
    message::GeneratorMessage,
    state::{Mode, TabKind},
    tabs,
};

impl GeneratorView {
    /// Returns `None` when the modal is closed so App's view composer can
    /// take a cheap exclusive branch (see CLAUDE.md → "Stack doesn't cull").
    pub fn modal_view<'a>(
        &'a self,
        ctx: &crate::app::RenderCtx<'a>,
    ) -> Option<Element<'a, GeneratorMessage, AppTheme>> {
        let progress = self.fade.progress_if_visible()?;

        let body = match self.mode {
            Mode::Generator => self.generator_body(ctx.colors),
            Mode::History => history::view(&self.history, ctx.colors),
        };

        let title_label = match self.mode {
            Mode::Generator => fl!("generator-title"),
            Mode::History => fl!("generator-history-title"),
        };
        let header = modal::dialog_header(title_label, GeneratorMessage::Close, ctx.colors);

        // Right-pad the body inside the scrollable so fields don't kiss
        // the rail. Match the cipher list's muted scroller styling so the
        // bar isn't a stark white sliver against the dark dialog.
        let scrolled = scrollable(
            container(body)
                .padding(Padding {
                    top: 0.0,
                    right: 12.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .width(Fill),
        )
        .height(Fill)
        .style(components::rail_scroll_style);

        let content = column![header, Space::new().height(16), scrolled,]
            .width(Fill)
            .height(Fill);

        // Right padding is reduced to compensate for the scrollable's
        // internal right-rail clearance (12px above), so cards sit
        // symmetrically inside the dialog instead of drifting left.
        let body = container(content)
            .padding(Padding {
                top: 12.0,
                right: 12.0,
                bottom: 12.0,
                left: 24.0,
            })
            .width(Fill)
            .height(Fill);

        Some(modal::dialog(
            680.0,
            Some(620.0),
            |c| c.card_bg,
            progress,
            body,
            GeneratorMessage::Close,
        ))
    }

    fn generator_body<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Element<'a, GeneratorMessage, AppTheme> {
        let tab_progress = self.tab_anim.animate_wrapped(Instant::now());
        let tabs = tab_row(self.active_tab, tab_progress, colors);
        let value_card = value_card(self.current.as_deref().unwrap_or(""), colors);
        let options: Element<'a, GeneratorMessage, AppTheme> = match self.active_tab {
            TabKind::Password => tabs::password::view(&self.password, colors),
            TabKind::Passphrase => tabs::passphrase::view(&self.passphrase, colors),
            TabKind::Username => tabs::username::view(&self.username, colors),
        };

        let history_row = history_entry_row(colors);

        column![
            tabs,
            Space::new().height(12),
            components::card_with_margin(value_card),
            options,
            history_row,
        ]
        .width(Fill)
        .into()
    }
}

// ── Shared render helpers ──────────────────────────────────────────────────

pub(super) use crate::components::section_heading;

/// Segmented tab bar (three buttons in a rounded pill). The accent pill
/// is a separate layer behind the buttons; its position interpolates
/// between tab indices via the caller-supplied `progress` (a float-valued
/// tab index driven by lilt). Buttons themselves render only their text.
fn tab_row<'a>(
    active: TabKind,
    progress: f32,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let n_tabs = TabKind::ALL.len();
    let max_index = (n_tabs - 1) as f32;
    let left = progress.clamp(0.0, max_index);
    let right = max_index - left;

    // FillPortion(0) collapses the space — clamp to a tiny share so the
    // sliding pill still renders a sliver at each end. The pill itself
    // gets a fixed share of `100`; the flanking spaces get `left * 100`
    // and `right * 100` shares so at integer `progress` values the pill
    // aligns exactly with the corresponding button.
    let pill_p: u16 = 100;
    let left_p = ((left * 100.0).round() as u16).max(1);
    let right_p = ((right * 100.0).round() as u16).max(1);

    let pill = container(Space::new())
        .width(Length::FillPortion(pill_p))
        .height(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.accent)
                .border(Border::default().rounded(RADIUS_PILL))
        });
    let indicator = row![
        Space::new().width(Length::FillPortion(left_p)),
        pill,
        Space::new().width(Length::FillPortion(right_p)),
    ]
    .height(Fill);

    let mut buttons_row = row![];
    for &k in TabKind::ALL {
        buttons_row = buttons_row.push(tab_button(k, k == active, colors));
    }

    // `push_under` puts the indicator behind the buttons without affecting
    // the stack's intrinsic size — the buttons row dictates height.
    let stacked = Stack::new().push(buttons_row).push_under(indicator);

    container(stacked)
        .padding(Padding::from([2, 2]))
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(Border::default().rounded(RADIUS_PILL))
        })
        .into()
}

fn tab_button<'a>(
    kind: TabKind,
    active: bool,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let label_color = if active {
        colors.text_primary
    } else {
        colors.text_secondary
    };

    let label = container(
        text(kind.label())
            .size(13)
            .color(label_color)
            .align_x(Alignment::Center),
    )
    .width(Fill)
    .align_x(Alignment::Center)
    .padding([4, 8]);

    // Transparent buttons — the sliding pill behind is the visual bg.
    // Hover is also transparent so it doesn't fight with the indicator.
    buttons::ghost(
        label,
        false,
        Color::TRANSPARENT,
        Color::TRANSPARENT,
        RADIUS_PILL,
    )
    .width(Fill)
    .on_press(GeneratorMessage::SelectTab(kind))
    .into()
}

/// Big value display with refresh + copy buttons on the right. Monospace
/// font for the value so long passwords align predictably.
fn value_card<'a>(
    value: &'a str,
    colors: &'a AppColors,
) -> Element<'a, GeneratorMessage, AppTheme> {
    let value_text = text(value.to_string())
        .size(16)
        .font(iced::Font::MONOSPACE)
        .color(colors.text_primary);

    let refresh = buttons::ghost_icon(
        icons::ARROW_REPEAT.render(18.0, colors.text_primary),
        colors.item_hover,
    )
    .padding([6, 6])
    .on_press(GeneratorMessage::Regenerate);

    let copy = buttons::ghost_icon(
        icons::BWI_COPY.render(18.0, colors.text_primary),
        colors.item_hover,
    )
    .padding([6, 6])
    .on_press(GeneratorMessage::CopyCurrent);

    components::styled_card(
        row![value_text, Space::new().width(Fill), refresh, copy]
            .align_y(Alignment::Center)
            .spacing(4),
    )
}

/// "Generator history >" disclosure row at the bottom of the generator
/// body. Tapping anywhere on the row flips the modal into history mode.
/// Styled as a clickable card matching the option cards so it sits
/// flush with the rest of the card-based layout.
fn history_entry_row<'a>(colors: &'a AppColors) -> Element<'a, GeneratorMessage, AppTheme> {
    let row_content = row![
        text(fl!("generator-history-open"))
            .size(14)
            .color(colors.accent)
            .font(crate::APP_FONT_BOLD),
        Space::new().width(Fill),
        icons::CHEVRON_RIGHT.render(14.0, colors.accent),
    ]
    .align_y(Alignment::Center);

    button(row_content)
        .width(Fill)
        .padding(Padding::from([12, 16]))
        .style(move |theme: &AppTheme, status| {
            let bg = match status {
                button::Status::Hovered => Background::Color(theme.colors.item_hover),
                _ => Background::Color(theme.colors.background),
            };
            button::Style {
                background: Some(bg),
                text_color: colors.text_primary,
                border: Border::default().rounded(RADIUS_LG),
                shadow: components::CARD_SHADOW,
                snap: false,
            }
        })
        .on_press(GeneratorMessage::ShowHistory)
        .into()
}

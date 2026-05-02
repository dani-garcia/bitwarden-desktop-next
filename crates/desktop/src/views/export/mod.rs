//! Export vault modal — wired to the sidebar Export button.
//!
//! Visual stub: format picker is real, but Submit only fires an
//! "unimplemented" toast. The banner shows the active user's email so the
//! "individual vault" message reads as concrete.

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExportFormat {
    pub id: &'static str,
    pub name: &'static str,
}

impl std::fmt::Display for ExportFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name)
    }
}

const EXPORT_FORMATS: &[ExportFormat] = &[
    ExportFormat { id: "json", name: ".json" },
    ExportFormat { id: "csv", name: ".csv" },
    ExportFormat { id: "encrypted_json", name: ".json (Encrypted)" },
];

const DEFAULT_FORMAT: ExportFormat = EXPORT_FORMATS[0];

// ── State ─────────────────────────────────────────────────────────────────

pub struct ExportView {
    pub fade: FadeInOut,
    /// Active user email — shown verbatim in the "Only items associated with"
    /// banner. Set via [`Self::open`] each time the modal is opened.
    email: String,
    selected_format: ExportFormat,
}

#[derive(Debug, Clone)]
pub enum ExportMessage {
    Close,
    Submit,
    FormatSelected(ExportFormat),
}

pub enum ExportEvent {
    Unimplemented,
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
            selected_format: DEFAULT_FORMAT,
        }
    }

    pub fn open(&mut self, email: String) {
        self.fade.open();
        self.email = email;
        self.selected_format = DEFAULT_FORMAT;
    }

    pub fn update(&mut self, msg: ExportMessage, _ctx: UpdateCtx<'_>) -> Outcome<Self> {
        match msg {
            ExportMessage::Close => {
                self.fade.close();
                Outcome::None
            }
            ExportMessage::Submit => Outcome::event(ExportEvent::Unimplemented),
            ExportMessage::FormatSelected(f) => {
                self.selected_format = f;
                Outcome::None
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

        // Tinted info banner — accent fill at low alpha so it reads as a
        // soft notification rather than a hard call-out.
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
            EXPORT_FORMATS.to_vec(),
            |f: &ExportFormat| f.name.to_string(),
            ExportMessage::FormatSelected,
            |c| c.card_bg,
            colors,
        );

        let submit_btn = buttons::primary(text(fl!("export-modal-submit")).size(14))
            .on_press(ExportMessage::Submit)
            .padding([8, 20]);
        let cancel_btn = buttons::secondary(text(fl!("export-modal-cancel")).size(14))
            .on_press(ExportMessage::Close)
            .padding([8, 20]);

        let body = column![
            header,
            banner,
            format_picker,
            row![submit_btn, cancel_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(16)
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

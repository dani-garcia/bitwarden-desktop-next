use bitwarden_send::SendType;
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding, Shadow,
    widget::{Space, checkbox, column, container, row, scrollable, text, text_editor, text_input},
};

use crate::{
    components::{buttons, icons, inputs, separator_h},
    fl,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

use super::{
    message::SendFormMessage,
    state::{AccessType, DeletionPreset, SendForm},
};

pub fn view<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, SendFormMessage, AppTheme> {
    let is_new = form.id.is_none();

    let title = match (is_new, form.send_type) {
        (true, SendType::Text) => fl!("send-form-title-new-text"),
        (true, SendType::File) => fl!("send-form-title-new-file"),
        (false, SendType::Text) => fl!("send-form-title-edit-text"),
        (false, SendType::File) => fl!("send-form-title-edit-file"),
    };

    let header = row![
        text(title)
            .size(18)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD)
            .width(Fill),
        buttons::ghost_icon(
            icons::BWI_CLOSE.render(16.0, colors.text_secondary),
            colors.item_hover,
        )
        .on_press(SendFormMessage::CancelPressed)
        .padding([6, 8]),
    ]
    .align_y(Alignment::Center);

    let header_container = container(header)
        .padding(Padding {
            top: 16.0,
            right: 20.0,
            bottom: 16.0,
            left: 20.0,
        })
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(iced::border::top(top_radius)))
        });

    let details = details_card(form, colors);
    let additional = additional_options_card(form, colors);

    let body: Element<'a, SendFormMessage, AppTheme> = scrollable(
        column![details, additional]
            .spacing(16)
            .padding(Padding {
                top: 16.0,
                right: 20.0,
                bottom: 16.0,
                left: 20.0,
            }),
    )
    .height(Fill)
    .style(scrollable_style)
    .into();

    let footer = footer(form, colors);

    container(column![header_container, separator_h(), body, footer].height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(iced::border::top(top_radius)))
        })
        .into()
}

// ── Cards ──────────────────────────────────────────────────────────────────

fn details_card<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut items: Vec<Element<'a, SendFormMessage, AppTheme>> = Vec::new();

    items.push(name_field(form, colors));

    match form.send_type {
        SendType::Text => {
            items.push(text_to_share_field(form, colors));
            items.push(
                checkbox(form.text_hidden)
                    .label(fl!("send-form-hide-text"))
                    .on_toggle(SendFormMessage::HideTextToggled)
                    .size(18)
                    .spacing(8)
                    .into(),
            );
        }
        SendType::File => {
            items.push(file_section(form, colors));
        }
    }

    items.push(deletion_date_field(form, colors));
    items.push(
        text(fl!(
            "send-form-deletion-hint",
            date = crate::views::send::widgets::send_list::format_deletion_date(
                &form.deletion_date
            )
        ))
        .size(12)
        .color(colors.text_secondary)
        .into(),
    );

    items.push(access_type_field(form, colors));

    if matches!(form.access_type, AccessType::Password) {
        items.push(
            text(fl!("send-form-password-hint"))
                .size(12)
                .color(colors.text_secondary)
                .into(),
        );
        items.push(password_field(form, colors));
    }

    if matches!(form.access_type, AccessType::People) {
        items.push(emails_field(form, colors));
    }

    if let Some(link) = form.send_link() {
        items.push(send_link_field(link, colors));
    }

    card_section(fl!("send-form-details-heading"), items, colors)
}

fn additional_options_card<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut items: Vec<Element<'a, SendFormMessage, AppTheme>> = Vec::new();
    items.push(limit_views_field(form, colors));
    items.push(
        views_left_hint(form, colors).unwrap_or_else(|| {
            text(fl!("send-form-limit-views-hint"))
                .size(12)
                .color(colors.text_secondary)
                .into()
        }),
    );
    items.push(
        checkbox(form.hide_email)
            .label(fl!("send-form-hide-email"))
            .on_toggle(SendFormMessage::HideEmailToggled)
            .size(18)
            .spacing(8)
            .into(),
    );
    items.push(notes_field(form, colors));

    card_section(fl!("send-form-additional-heading"), items, colors)
}

fn card_section<'a>(
    heading: String,
    items: Vec<Element<'a, SendFormMessage, AppTheme>>,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let body = container(column(items).spacing(12))
        .padding(16)
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(Border::default().rounded(RADIUS_LG))
                .shadow(Shadow {
                    color: Color::from_rgba(0.0, 0.0, 0.0, 0.15),
                    offset: iced::Vector::new(0.0, 1.0),
                    blur_radius: 2.0,
                })
        });

    column![
        text(heading)
            .size(14)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD),
        body,
    ]
    .spacing(8)
    .into()
}

// ── Individual fields ──────────────────────────────────────────────────────

fn name_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut input = inputs::bare_text_input(&form.name);
    if !form.saving {
        input = input.on_input(SendFormMessage::NameChanged);
    }
    inputs::field_frame(fl!("send-form-name"), input.into(), colors)
}

fn text_to_share_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut editor = text_editor(&form.text_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(80.0)
        .max_height(240.0);
    if !form.saving {
        editor = editor.on_action(SendFormMessage::TextAction);
    }
    inputs::field_frame(fl!("send-form-text"), editor.into(), colors)
}

fn file_section<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let is_new = form.id.is_none();
    if is_new {
        // New file send: the filename + size are "chosen" via a picker.
        // The picker is stubbed — see docs/todo.md.
        let placeholder = text(fl!("send-form-file-choose-placeholder"))
            .size(14)
            .color(colors.text_secondary);
        let choose_btn = buttons::secondary(
            text(fl!("send-form-file-choose")).size(14),
        )
        .on_press(SendFormMessage::ChooseFilePressed)
        .padding([8, 16]);
        row![placeholder, Space::new().width(Fill), choose_btn]
            .spacing(12)
            .align_y(Alignment::Center)
            .into()
    } else {
        // Editing existing file: show name + size, non-editable. The
        // callbacks are no-ops since `disabled: true` blocks input at the
        // widget layer — a single uniform `|_|` keeps the closure types
        // identical across both fields so iced's type inference settles
        // on one `Element` type.
        let noop = |_: String| SendFormMessage::ChooseFilePressed;
        let name = inputs::text_field(
            fl!("send-form-file-name"),
            &form.file_name,
            noop,
            None,
            true,
            colors,
        );
        let size = inputs::text_field(
            fl!("send-form-file-size"),
            form.file_size_name.as_deref().unwrap_or(""),
            noop,
            None,
            true,
            colors,
        );
        column![name, size].spacing(12).into()
    }
}

fn deletion_date_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    inputs::select_field(
        fl!("send-form-deletion-date"),
        Some(form.deletion_preset),
        DeletionPreset::ALL.to_vec(),
        |preset| preset_label(*preset),
        SendFormMessage::DeletionPresetChosen,
        colors,
    )
}

fn access_type_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    inputs::select_field(
        fl!("send-form-who-can-view"),
        Some(form.access_type),
        AccessType::ALL.to_vec(),
        |t| access_type_label(*t),
        SendFormMessage::AccessTypeChosen,
        colors,
    )
}

fn password_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut input =
        inputs::bare_text_input(&form.password).on_input(SendFormMessage::PasswordChanged);
    if !form.password_revealed {
        input = input.secure(true);
    }

    let eye_icon = if form.password_revealed {
        icons::EYE
    } else {
        icons::EYE_SLASH
    };
    let eye_btn = buttons::ghost_icon(
        eye_icon.render(16.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::PasswordRevealToggled)
    .padding([10, 8]);

    let regen_btn = buttons::ghost_icon(
        icons::ARROW_CLOCKWISE.render(16.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::PasswordRegenerate)
    .padding([10, 8]);

    let copy_btn = buttons::ghost_icon(
        icons::COPY.render(16.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::PasswordCopy)
    .padding([10, 8]);

    let row_el = row![input, eye_btn, regen_btn, copy_btn]
        .align_y(Alignment::Center);

    inputs::field_frame(fl!("send-form-password"), row_el.into(), colors)
}

fn emails_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut editor = text_editor(&form.emails_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(60.0)
        .max_height(140.0);
    if !form.saving {
        editor = editor.on_action(SendFormMessage::EmailsAction);
    }
    inputs::field_frame(fl!("send-form-emails"), editor.into(), colors)
}

fn send_link_field<'a>(
    url: String,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let input = text_input("", &url)
        .size(14)
        .padding([10, 12])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });
    let copy = buttons::ghost_icon(
        icons::COPY.render(16.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::CopyLinkPressed)
    .padding([10, 8]);
    let row_el = row![input, copy].align_y(Alignment::Center);
    inputs::field_frame(fl!("send-form-send-link"), row_el.into(), colors)
}

fn limit_views_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let input = inputs::bare_text_input(&form.max_access_count_raw)
        .on_input(SendFormMessage::MaxAccessCountChanged);
    let inc = buttons::ghost_icon(
        icons::CHEVRON_UP.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::MaxAccessCountIncrement)
    .padding([2, 6]);
    let dec = buttons::ghost_icon(
        icons::CHEVRON_DOWN.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(SendFormMessage::MaxAccessCountDecrement)
    .padding([2, 6]);

    let steppers = column![inc, dec].spacing(0);
    let row_el = row![input, steppers].align_y(Alignment::Center);
    inputs::field_frame(fl!("send-form-limit-views"), row_el.into(), colors)
}

fn views_left_hint<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Option<Element<'a, SendFormMessage, AppTheme>> {
    let left = form.views_left()?;
    Some(
        text(fl!(
            "send-form-views-left",
            count = left.to_string()
        ))
        .size(12)
        .color(colors.text_secondary)
        .into(),
    )
}

fn notes_field<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut editor = text_editor(&form.notes_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(80.0)
        .max_height(240.0);
    if !form.saving {
        editor = editor.on_action(SendFormMessage::NotesAction);
    }
    inputs::field_frame(fl!("send-form-private-note"), editor.into(), colors)
}

// ── Footer (Save / Cancel / Delete) ────────────────────────────────────────

fn footer<'a>(
    form: &'a SendForm,
    colors: &'a AppColors,
) -> Element<'a, SendFormMessage, AppTheme> {
    let mut save = buttons::primary(text(fl!("send-form-save")).size(14))
        .padding([8, 20]);
    if form.is_valid() && !form.saving {
        save = save.on_press(SendFormMessage::SavePressed);
    }

    let cancel = buttons::secondary(text(fl!("send-form-cancel")).size(14))
        .on_press(SendFormMessage::CancelPressed)
        .padding([8, 20]);

    let mut left_row = row![save, cancel].spacing(8).align_y(Alignment::Center);

    // Delete icon only for existing sends — there's no saved resource to
    // remove on a brand-new draft. Uses `titlebar_close_hover` (red) for
    // consistency with the vault detail pane's trash icon.
    if form.id.is_some() {
        let delete_btn = buttons::ghost_icon(
            icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
            colors.item_hover,
        )
        .on_press(SendFormMessage::DeletePressed)
        .padding([6, 8]);
        left_row = left_row.push(Space::new().width(Fill));
        left_row = left_row.push(delete_btn);
    }

    container(left_row)
        .padding(Padding {
            top: 12.0,
            right: 20.0,
            bottom: 12.0,
            left: 20.0,
        })
        .width(Fill)
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0),
                )
        })
        .into()
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn access_type_label(t: AccessType) -> String {
    match t {
        AccessType::Link => fl!("send-form-access-link"),
        AccessType::People => fl!("send-form-access-people"),
        AccessType::Password => fl!("send-form-access-password"),
    }
}

fn preset_label(p: DeletionPreset) -> String {
    match p {
        DeletionPreset::OneHour => fl!("send-form-preset-1h"),
        DeletionPreset::OneDay => fl!("send-form-preset-1d"),
        DeletionPreset::TwoDays => fl!("send-form-preset-2d"),
        DeletionPreset::ThreeDays => fl!("send-form-preset-3d"),
        DeletionPreset::SevenDays => fl!("send-form-preset-7d"),
        DeletionPreset::FourteenDays => fl!("send-form-preset-14d"),
        DeletionPreset::ThirtyDays => fl!("send-form-preset-30d"),
    }
}

fn scrollable_style(
    theme: &AppTheme,
    _status: scrollable::Status,
) -> scrollable::Style {
    scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(theme.colors.item_hover),
                border: Border::default().rounded(4),
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(theme.colors.item_hover),
                border: Border::default().rounded(4),
            },
        },
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    }
}


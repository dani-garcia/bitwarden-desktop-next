//! Reusable labeled input primitives shared across views.
//!
//! Naming convention: every public helper ends in `_field` (or is
//! [`field_frame`], the shared visual primitive). Fields with a show/hide
//! eye toggle are prefixed `reveal_*` — `reveal_text_field` for editable
//! inputs and `reveal_field` for read-only display. All widgets compose
//! through [`field_frame`] so the floating-label chip looks identical
//! regardless of what's inside.

#![allow(deprecated)] // `Component` is deprecation-tagged upstream; see `reveal_text_field` docs.

use std::fmt::Display;

use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length,
    widget::{
        Component, Space, column, combo_box, component, container, pick_list, row, stack, text,
        text_input, TextInput,
    },
};

use crate::{
    components::{buttons, drop_down::DropDown, icons},
    theme::{AppColors, AppTheme},
};

/// Preconfigured `text_input` matching our floating-label look:
/// transparent background + no border (the wrapping [`field_frame`] draws
/// the border). Callers chain `.on_input`/`.on_submit`/`.secure` as needed.
pub fn bare_text_input<'a, M: Clone + 'a>(value: &'a str) -> TextInput<'a, M, AppTheme> {
    text_input("", value)
        .size(16)
        .padding([10, 12])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        })
}

/// Wrap any content in the floating-label frame: a bordered container with
/// a small label chip stacked on top of the border. The chip has a
/// background matching the page, so the border visually breaks behind it.
pub fn field_frame<'a, M: 'a>(
    label: &'a str,
    content: Element<'a, M, AppTheme>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    let floating_label = container(text(label).size(14).color(colors.text_secondary))
        .padding([0, 4])
        .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background));

    let bordered = container(content).width(Fill).style(|theme: &AppTheme| {
        container::Style::default().border(
            Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(4),
        )
    });

    stack![
        column![Space::new().height(Length::Fixed(8.0)), bordered],
        container(floating_label).padding([0, 12]),
    ]
    .into()
}

/// Reusable labeled text input.
pub fn text_field<'a, M>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: Option<M>,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let mut input = bare_text_input(value);

    // Skipping `.on_input` leaves the text_input read-only (iced renders it
    // as non-editable). We also drop `.on_submit` so Enter-spam during any
    // in-flight task is ignored at the widget layer.
    if !disabled {
        input = input.on_input(on_input);
        if let Some(submit_msg) = on_submit {
            input = input.on_submit(submit_msg);
        }
    }

    field_frame(label, input.into(), colors)
}

/// Labeled single-select dropdown backed by iced's `pick_list`.
///
/// The inner `pick_list` draws its own border via `pick_list::Catalog`, so
/// we null it out and let [`field_frame`] own the border — that's what makes
/// the label chip cleanly "cut" the top edge.
pub fn select_field<'a, T, M>(
    label: &'a str,
    selected: Option<T>,
    options: Vec<T>,
    to_string: impl Fn(&T) -> String + 'a,
    on_select: impl Fn(T) -> M + 'a,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    T: PartialEq + Clone + 'a,
    M: Clone + 'a,
{
    let picker = pick_list(selected, options, to_string)
        .on_select(on_select)
        .width(Fill)
        .padding([8, 12])
        .style(
            |theme: &AppTheme, _status| iced::widget::pick_list::Style {
                text_color: theme.colors.text_primary,
                background: Background::Color(Color::TRANSPARENT),
                placeholder_color: theme.colors.text_secondary,
                handle_color: theme.colors.text_secondary,
                border: Border::default(),
            },
        );

    field_frame(label, picker.into(), colors)
}

/// Labeled searchable single-select backed by iced's `combo_box` — user
/// types to filter the option list.
///
/// `combo_box::State<T>` has to stay on the parent struct (iced requires
/// `&'a combo_box::State<T>` at render time and the options are loaded
/// asynchronously), but the open/closed flag and caret direction live
/// inside this Component. `on_close` is emitted when the combo_box loses
/// focus so the parent can rebuild `combo_box::State` to clear the
/// `value` field — iced exposes no public API to reset it otherwise, and
/// leaving it populated would re-filter on the next open.
pub fn search_select_field<'a, T, M>(
    state: &'a combo_box::State<T>,
    label: &'a str,
    placeholder: String,
    selected: Option<T>,
    on_selected: impl Fn(T) -> M + 'a,
    on_close: M,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    T: Display + Clone + 'static,
    M: Clone + 'a,
{
    component(SearchSelectField {
        state,
        label,
        placeholder,
        selected,
        on_selected: Box::new(on_selected),
        on_close,
        colors,
    })
}

struct SearchSelectField<'a, T, M> {
    state: &'a combo_box::State<T>,
    label: &'a str,
    placeholder: String,
    selected: Option<T>,
    on_selected: Box<dyn Fn(T) -> M + 'a>,
    on_close: M,
    colors: &'a AppColors,
}

#[derive(Default)]
struct SearchSelectFieldState {
    is_open: bool,
}

#[derive(Clone)]
enum SearchSelectFieldEvent<T> {
    Selected(T),
    Opened,
    Closed,
}

impl<'a, T, M> Component<M, AppTheme> for SearchSelectField<'a, T, M>
where
    T: Display + Clone + 'static,
    M: Clone + 'a,
{
    type State = SearchSelectFieldState;
    type Event = SearchSelectFieldEvent<T>;

    fn update(&mut self, state: &mut Self::State, event: Self::Event) -> Option<M> {
        match event {
            SearchSelectFieldEvent::Selected(t) => Some((self.on_selected)(t)),
            SearchSelectFieldEvent::Opened => {
                state.is_open = true;
                None
            }
            SearchSelectFieldEvent::Closed => {
                state.is_open = false;
                Some(self.on_close.clone())
            }
        }
    }

    fn view(&self, state: &Self::State) -> Element<'_, Self::Event, AppTheme> {
        let caret = if state.is_open {
            icons::CHEVRON_UP
        } else {
            icons::CHEVRON_DOWN
        }
        .input_icon(14.0, text_input::Side::Right);

        let picker = combo_box(
            self.state,
            &self.placeholder,
            self.selected.as_ref(),
            SearchSelectFieldEvent::Selected,
        )
        .width(Fill)
        .padding([8, 12])
        .icon(caret)
        .on_open(SearchSelectFieldEvent::Opened)
        .on_close(SearchSelectFieldEvent::Closed)
        .input_style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            icon: theme.colors.text_secondary,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });

        field_frame(self.label, picker.into(), self.colors)
    }
}

/// Labeled wrapper around our custom [`DropDown`] — for multi-select panels
/// (e.g. collections) or any dropdown whose trigger/panel callers want to
/// render themselves.
pub fn multi_select_field<'a, M: Clone + 'a>(
    label: &'a str,
    trigger: Element<'a, M, AppTheme>,
    panel: Element<'a, M, AppTheme>,
    open: bool,
    on_dismiss: M,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    // Don't set a width on the DropDown — the overlay defaults to the
    // trigger's width (see `drop_down.rs` layout). `Length::Fill` would
    // stretch the overlay to the whole window.
    let dd: Element<'a, M, AppTheme> = DropDown::new(trigger, panel, open)
        .alignment(crate::components::drop_down::Alignment::BelowLeft)
        .on_dismiss(on_dismiss)
        .offset(4.0)
        .into();

    field_frame(label, dd, colors)
}

// ─── reveal_* : fields with an eye toggle ──────────────────────────────────
//
// `Component` is deprecation-tagged upstream, but the deprecation is
// philosophical — the replacement is hand-rolling `Widget`, which for a
// compound of `text_input` + button means reimplementing the bridge we
// get here for free. Component is the right tool for widgets that genuinely
// *should* own their transient state (whether a password is currently
// visible has no meaning outside the widget). If it ever disappears
// upstream we can swap in a hand-rolled `Widget`.

/// Build a password/hidden-value input with self-managed reveal state.
///
/// `label` is the floating chip. `value` is the current text. `on_input` is
/// fired on every keystroke. `disabled=true` makes the text input read-only
/// and the toggle inert.
pub fn reveal_text_field<'a, Message>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealTextField {
        label,
        value,
        on_input: Box::new(on_input),
        on_submit: None,
        disabled,
        colors,
    })
}

/// Variant of [`reveal_text_field`] that also fires `on_submit` when the user
/// presses Enter inside the field.
pub fn reveal_text_field_with_submit<'a, Message>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    on_submit: Message,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealTextField {
        label,
        value,
        on_input: Box::new(on_input),
        on_submit: Some(on_submit),
        disabled,
        colors,
    })
}

struct RevealTextField<'a, Message> {
    label: &'a str,
    value: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_submit: Option<Message>,
    disabled: bool,
    colors: &'a AppColors,
}

#[derive(Default)]
struct RevealTextState {
    revealed: bool,
}

#[derive(Clone)]
enum RevealTextEvent {
    Input(String),
    Submit,
    Toggle,
}

impl<'a, Message: Clone + 'a> Component<Message, AppTheme> for RevealTextField<'a, Message> {
    type State = RevealTextState;
    type Event = RevealTextEvent;

    fn update(&mut self, state: &mut RevealTextState, event: RevealTextEvent) -> Option<Message> {
        match event {
            RevealTextEvent::Input(s) => Some((self.on_input)(s)),
            RevealTextEvent::Submit => self.on_submit.clone(),
            RevealTextEvent::Toggle => {
                state.revealed = !state.revealed;
                None
            }
        }
    }

    fn view(&self, state: &RevealTextState) -> Element<'_, RevealTextEvent, AppTheme> {
        let mut input = bare_text_input(self.value);
        if !self.disabled {
            input = input.on_input(RevealTextEvent::Input);
            if self.on_submit.is_some() {
                input = input.on_submit(RevealTextEvent::Submit);
            }
        }
        if !state.revealed {
            input = input.secure(true);
        }

        let eye_icon = if state.revealed {
            icons::EYE
        } else {
            icons::EYE_SLASH
        }
        .render(16.0, self.colors.text_secondary);

        let mut toggle_button =
            buttons::ghost_icon(eye_icon, self.colors.item_hover).padding([10, 12]);
        if !self.disabled {
            toggle_button = toggle_button.on_press(RevealTextEvent::Toggle);
        }

        let input_row = row![input, toggle_button].align_y(Alignment::Center);
        field_frame(self.label, input_row.into(), self.colors)
    }
}

/// Read-only labeled field with a self-managed eye toggle.
///
/// Displays `value` as bullets by default; clicking the eye reveals the
/// real text. Optional `on_copy` appends a copy icon next to the eye.
/// Unlike [`reveal_text_field`], this renders static `text(...)` (not
/// `text_input`) — intended for the detail pane where fields are not
/// editable but still secret.
pub fn reveal_field<'a, Message>(
    label: &'a str,
    value: &'a str,
    on_copy: Option<Message>,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealField {
        label,
        value,
        on_copy,
        colors,
    })
}

struct RevealField<'a, Message> {
    label: &'a str,
    value: &'a str,
    on_copy: Option<Message>,
    colors: &'a AppColors,
}

#[derive(Default)]
struct RevealFieldState {
    revealed: bool,
}

#[derive(Clone)]
enum RevealFieldEvent {
    Toggle,
    Copy,
}

impl<'a, Message: Clone + 'a> Component<Message, AppTheme> for RevealField<'a, Message> {
    type State = RevealFieldState;
    type Event = RevealFieldEvent;

    fn update(&mut self, state: &mut RevealFieldState, event: RevealFieldEvent) -> Option<Message> {
        match event {
            RevealFieldEvent::Toggle => {
                state.revealed = !state.revealed;
                None
            }
            RevealFieldEvent::Copy => self.on_copy.clone(),
        }
    }

    fn view(&self, state: &RevealFieldState) -> Element<'_, RevealFieldEvent, AppTheme> {
        // Match the bullet count to the real value's length (clamped) so the
        // row width doesn't visibly jump when toggling.
        let display: String = if state.revealed {
            self.value.to_string()
        } else {
            "\u{2022}".repeat(self.value.chars().count().clamp(1, 24))
        };

        let eye_icon = if state.revealed {
            icons::BWI_EYE_SLASH
        } else {
            icons::BWI_EYE
        };
        let eye_button =
            buttons::ghost_icon(eye_icon.render(18.0, self.colors.text_primary), self.colors.item_hover)
                .padding([6, 6])
                .on_press(RevealFieldEvent::Toggle);

        let mut buttons_row: Vec<Element<'_, RevealFieldEvent, AppTheme>> = vec![eye_button.into()];
        if self.on_copy.is_some() {
            let copy_button = buttons::ghost_icon(
                icons::BWI_COPY.render(18.0, self.colors.text_primary),
                self.colors.item_hover,
            )
            .padding([6, 6])
            .on_press(RevealFieldEvent::Copy);
            buttons_row.push(copy_button.into());
        }

        row![
            column![
                text(self.label).size(12).color(self.colors.text_muted),
                text(display).size(14).color(self.colors.text_primary),
            ]
            .spacing(2)
            .width(Fill),
            row(buttons_row).spacing(2).align_y(Alignment::Center),
        ]
        .spacing(4)
        .align_y(Alignment::Center)
        .into()
    }
}

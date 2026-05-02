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
        Component, Space, TextInput, column, combo_box, component, container, pick_list, row,
        stack, text,
        text::{Ellipsis, Wrapping},
        text_input,
    },
};

use crate::{
    components::{buttons, drop_down::DropDown, icons},
    theme::{AppColors, AppTheme},
};

/// Preconfigured `text_input` matching our floating-label look:
/// transparent background + no border (the wrapping [`field_frame`] draws
/// the border).
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

/// Stacked label + single-line truncated value. Read-only display for long
/// unbroken strings (SSH keys, URIs, hashes, fingerprints) — a normal
/// `text()` would wrap on word boundaries or overflow the container. The
/// caller should provide a copy action alongside.
pub fn readonly_field_truncated<'a, M: 'a>(
    label: impl Into<String>,
    value: impl iced::widget::text::IntoFragment<'a>,
    colors: &AppColors,
) -> Element<'a, M, AppTheme> {
    column![
        text(label.into()).size(12).color(colors.text_muted),
        text(value)
            .size(14)
            .color(colors.text_primary)
            .wrapping(Wrapping::None)
            .ellipsis(Ellipsis::End),
    ]
    .spacing(2)
    // `Fill` is required for ellipsis to kick in — without it the text
    // widget is content-sized and lays out the full un-truncated string.
    .width(Fill)
    .into()
}

/// Wrap any content in the floating-label frame: a bordered container with
/// a small label chip stacked on top of the border. The chip's background
/// must match whatever surface sits behind the field so the border visually
/// breaks behind it — pass [`field_frame_on`] if the field doesn't sit on
/// `colors.background`.
///
/// The output is wrapped in a [`ShellScope`] so each field gets its own
/// private event-status flag — without it, two `pick_list`-bearing
/// `field_frame`s as siblings interact badly: iced's `Stack` short-circuits
/// its between-children iteration on the *shared* shell's
/// `is_event_captured()`, so a capture inside the first field's stack
/// stops the second field's stack from reaching its picker. Result: open
/// the second pick_list, click the first, and both end up open. See
/// [`crate::components::shell_scope`] for the full write-up.
pub fn field_frame<'a, M: 'a>(
    label: impl Into<String>,
    content: impl Into<Element<'a, M, AppTheme>>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    field_frame_on(label, content, |c| c.background, colors)
}

/// [`field_frame`] with a caller-chosen chip background. Use when the
/// field sits on a surface other than `colors.background` (e.g. directly
/// on a dialog whose body uses `card_bg`); pass the same colour token the
/// surrounding container paints.
///
/// `chip_bg` is a closure rather than a pre-resolved `Color` so the chip
/// re-themes correctly on light/dark switches.
pub fn field_frame_on<'a, M: 'a>(
    label: impl Into<String>,
    content: impl Into<Element<'a, M, AppTheme>>,
    chip_bg: impl Fn(&AppColors) -> Color + 'static,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    let floating_label = container(text(label.into()).size(14).color(colors.text_secondary))
        .padding([0, 4])
        .style(move |theme: &AppTheme| {
            container::Style::default().background(chip_bg(&theme.colors))
        });

    let bordered = container(content).width(Fill).style(|theme: &AppTheme| {
        container::Style::default().border(
            Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(4),
        )
    });

    crate::components::shell_scope::ShellScope::new(stack![
        column![Space::new().height(Length::Fixed(8.0)), bordered],
        container(floating_label).padding([0, 12]),
    ])
    .into()
}

/// [`field_frame`] in the validation-error state: red border + red label
/// chip. Pair with an inline error message rendered separately (e.g. via
/// [`field_error_row`]) so the user knows what failed validation.
pub fn errored_field_frame<'a, M: 'a>(
    label: impl Into<String>,
    content: impl Into<Element<'a, M, AppTheme>>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    let floating_label = container(text(label.into()).size(14).color(colors.danger))
        .padding([0, 4])
        .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background));

    let bordered = container(content).width(Fill).style(|theme: &AppTheme| {
        container::Style::default().border(
            Border::default()
                .color(theme.colors.danger)
                .width(1.0)
                .rounded(4),
        )
    });

    crate::components::shell_scope::ShellScope::new(stack![
        column![Space::new().height(Length::Fixed(8.0)), bordered],
        container(floating_label).padding([0, 12]),
    ])
    .into()
}

/// Inline validation-error row: filled X-circle icon + message text, both in
/// `colors.danger`. Render directly below an [`errored_field_frame`].
pub fn field_error_row<'a, M: 'a>(
    message: impl Into<String>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    row![
        icons::X_CIRCLE_FILL.render(14.0, colors.danger),
        text(message.into()).size(12).color(colors.danger),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

/// Labeled single-line text input — the workhorse field builder.
///
/// Returns a [`TextField`] for chaining; convert to an `Element` via
/// `.into()` (or implicitly via `column!` / `row!`). Mirrors iced's own
/// `button(content).on_press(msg)` idiom so the absence of a knob means
/// the default: skip `.on_input` for a readonly field, skip `.on_submit`
/// to ignore Enter, leave `.disabled(false)` and `.errored(false)` alone.
///
/// Closures (`on_input`, `chip_bg`) are boxed so the struct stays
/// non-generic over the closure type — the heap pointer per field is
/// negligible next to the iced widget tree it produces.
pub fn text_field<'a, M>(
    label: impl Into<String>,
    value: &'a str,
    colors: &'a AppColors,
) -> TextField<'a, M>
where
    M: Clone + 'a,
{
    TextField {
        label: label.into(),
        value,
        colors,
        on_input: None,
        on_submit: None,
        disabled: false,
        errored: false,
        chip_bg: None,
        id: None,
    }
}

type ChipBgFn = Box<dyn Fn(&AppColors) -> Color + 'static>;

pub struct TextField<'a, M> {
    label: String,
    value: &'a str,
    colors: &'a AppColors,
    on_input: Option<Box<dyn Fn(String) -> M + 'a>>,
    on_submit: Option<M>,
    disabled: bool,
    errored: bool,
    /// Defaults to `|c| c.background`. Set when the field sits on a
    /// surface other than `colors.background` so the floating-label chip
    /// blends. Folds in the `field_frame_on` variant.
    chip_bg: Option<ChipBgFn>,
    id: Option<iced::widget::Id>,
}

impl<'a, M: Clone + 'a> TextField<'a, M> {
    pub fn on_input(mut self, on_input: impl Fn(String) -> M + 'a) -> Self {
        self.on_input = Some(Box::new(on_input));
        self
    }

    pub fn on_submit(mut self, on_submit: M) -> Self {
        self.on_submit = Some(on_submit);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn errored(mut self, errored: bool) -> Self {
        self.errored = errored;
        self
    }

    pub fn chip_bg(mut self, chip_bg: impl Fn(&AppColors) -> Color + 'static) -> Self {
        self.chip_bg = Some(Box::new(chip_bg));
        self
    }

    pub fn id(mut self, id: iced::widget::Id) -> Self {
        self.id = Some(id);
        self
    }
}

impl<'a, M: Clone + 'a> From<TextField<'a, M>> for Element<'a, M, AppTheme> {
    fn from(field: TextField<'a, M>) -> Self {
        let TextField {
            label,
            value,
            colors,
            on_input,
            on_submit,
            disabled,
            errored,
            chip_bg,
            id,
        } = field;

        let mut input = bare_text_input(value);
        if let Some(id) = id {
            input = input.id(id);
        }
        // Skipping `.on_input` leaves the text_input read-only. Dropping
        // `.on_submit` ignores Enter-spam during in-flight tasks.
        if !disabled && let Some(on_input) = on_input {
            input = input.on_input(on_input);
            if let Some(submit_msg) = on_submit {
                input = input.on_submit(submit_msg);
            }
        }

        let content: Element<'a, M, AppTheme> = input.into();
        if errored {
            errored_field_frame(label, content, colors)
        } else if let Some(chip_bg) = chip_bg {
            field_frame_on(label, content, move |c| chip_bg(c), colors)
        } else {
            field_frame(label, content, colors)
        }
    }
}

/// Number input with up/down chevron steppers on the right edge. Caller
/// is responsible for rejecting non-digit `on_input` values and clamping
/// the deltas. `disabled = true` strips both `on_input` and the chevron
/// `on_press` handlers.
pub fn stepper_field<'a, M>(
    label: impl Into<String>,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    on_increment: M,
    on_decrement: M,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let mut input = bare_text_input(value);
    if !disabled {
        input = input.on_input(on_input);
    }

    let mut inc = buttons::ghost_icon(
        icons::CHEVRON_UP.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .padding([2, 6]);
    let mut dec = buttons::ghost_icon(
        icons::CHEVRON_DOWN.render(11.0, colors.text_secondary),
        colors.item_hover,
    )
    .padding([2, 6]);

    if !disabled {
        inc = inc.on_press(on_increment);
        dec = dec.on_press(on_decrement);
    }

    let steppers = column![inc, dec].spacing(0);
    let row_el = row![input, steppers].align_y(Alignment::Center);
    field_frame(label, row_el, colors)
}

/// Labeled single-select dropdown backed by iced's `pick_list`. The inner
/// `pick_list`'s border is nulled out so [`field_frame`] owns the border
/// and the label chip can cut the top edge cleanly.
pub fn select_field<'a, T, M>(
    label: impl Into<String>,
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
    select_field_on(
        label,
        selected,
        options,
        to_string,
        on_select,
        |c| c.background,
        colors,
    )
}

/// [`select_field`] with a caller-chosen chip background. Use when the
/// field doesn't sit on `colors.background` (e.g. directly on a dialog
/// whose body uses `card_bg`).
pub fn select_field_on<'a, T, M>(
    label: impl Into<String>,
    selected: Option<T>,
    options: Vec<T>,
    to_string: impl Fn(&T) -> String + 'a,
    on_select: impl Fn(T) -> M + 'a,
    chip_bg: impl Fn(&AppColors) -> Color + 'static,
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
        .style(|theme: &AppTheme, _status| iced::widget::pick_list::Style {
            text_color: theme.colors.text_primary,
            background: Background::Color(Color::TRANSPARENT),
            placeholder_color: theme.colors.text_secondary,
            handle_color: theme.colors.text_secondary,
            border: Border::default(),
        });

    field_frame_on(label, picker, chip_bg, colors)
}

/// Labeled searchable single-select backed by iced's `combo_box`.
///
/// `combo_box::State<T>` has to stay on the parent struct, but the
/// open/closed flag and caret direction live inside this Component.
/// `on_close` fires on focus loss so the parent can rebuild
/// `combo_box::State` to clear `value` — iced exposes no public API to
/// reset it, and leaving it populated would re-filter on the next open.
pub fn search_select_field<'a, T, M>(
    state: &'a combo_box::State<T>,
    label: impl Into<String>,
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
        label: label.into(),
        placeholder,
        selected,
        on_selected: Box::new(on_selected),
        on_close,
        colors,
    })
}

struct SearchSelectField<'a, T, M> {
    state: &'a combo_box::State<T>,
    label: String,
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

        field_frame(self.label.clone(), picker, self.colors)
    }
}

/// Labeled wrapper around our custom [`DropDown`] — for multi-select panels
/// (e.g. collections) or any dropdown whose trigger/panel callers want to
/// render themselves.
pub fn multi_select_field<'a, M: Clone + 'a>(
    label: impl Into<String>,
    trigger: impl Into<Element<'a, M, AppTheme>>,
    panel: impl Into<Element<'a, M, AppTheme>>,
    open: bool,
    on_dismiss: M,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    // Don't set a width on the DropDown — the overlay defaults to the
    // trigger's width. `Length::Fill` would stretch it to the whole window.
    let dd = DropDown::new(trigger.into(), panel.into(), open)
        .alignment(crate::components::drop_down::Alignment::BelowLeft)
        .on_dismiss(on_dismiss)
        .offset(4.0);

    field_frame(label, dd, colors)
}

// ─── reveal_* : fields with an eye toggle ──────────────────────────────────
//
// `Component` is deprecation-tagged upstream, but the replacement is
// hand-rolling `Widget`, which for a compound of `text_input` + button
// means reimplementing the bridge we get here for free. Component is the
// right tool for widgets that genuinely should own their transient state.

/// Build a password/hidden-value input with self-managed reveal state.
pub fn reveal_text_field<'a, Message>(
    label: impl Into<String>,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealTextField {
        id: None,
        label: label.into(),
        value,
        on_input: Box::new(on_input),
        on_submit: None,
        disabled,
        colors,
    })
}

/// Variant of [`reveal_text_field`] that fires `on_submit` on Enter. `id`
/// lets the caller target the inner `text_input` via
/// `widget::operation::focus(id)`.
pub fn reveal_text_field_with_submit<'a, Message>(
    id: Option<iced::widget::Id>,
    label: impl Into<String>,
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
        id,
        label: label.into(),
        value,
        on_input: Box::new(on_input),
        on_submit: Some(on_submit),
        disabled,
        colors,
    })
}

struct RevealTextField<'a, Message> {
    id: Option<iced::widget::Id>,
    label: String,
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
        if let Some(id) = self.id.clone() {
            input = input.id(id);
        }
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
        field_frame(self.label.clone(), input_row, self.colors)
    }
}

/// Read-only labeled field with a self-managed eye toggle. Displays
/// `value` as bullets by default; the eye reveals the real text. Unlike
/// [`reveal_text_field`], this renders static `text(...)` — intended for
/// the detail pane where fields are not editable but still secret.
pub fn reveal_field<'a, Message>(
    label: impl Into<String>,
    value: &'a str,
    on_copy: Option<Message>,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealField {
        label: label.into(),
        value,
        on_copy,
        colors,
    })
}

struct RevealField<'a, Message> {
    label: String,
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
        // Match bullet count to the value's length (clamped) so the row
        // width doesn't visibly jump when toggling.
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
        let eye_button = buttons::ghost_icon(
            eye_icon.render(18.0, self.colors.text_primary),
            self.colors.item_hover,
        )
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
            readonly_field_truncated(self.label.clone(), display, self.colors),
            row(buttons_row).spacing(2).align_y(Alignment::Center),
        ]
        .spacing(4)
        .width(Fill)
        .align_y(Alignment::Center)
        .into()
    }
}

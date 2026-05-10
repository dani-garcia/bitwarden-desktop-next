//! Fields with an eye toggle. `Component` is deprecation-tagged upstream,
//! but the replacement is hand-rolling `Widget`, which for a compound of
//! `text_input` + button means reimplementing the bridge we get here for
//! free. Component is the right tool for widgets that genuinely should
//! own their transient state.

use iced::{
    Alignment, Element, Fill,
    widget::{Component, component, row},
};

use crate::{
    components::{buttons, icons},
    theme::{AppColors, AppTheme},
};

use super::{bare_text_input, field_frame, readonly_field_truncated};

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

        // Match the read-only `reveal_field` variant: BWI font, 18 px,
        // text_primary, "show what clicking does" convention (BWI_EYE = click
        // to reveal). Otherwise the same form would render two different
        // glyph families for the same affordance.
        let eye_icon = if state.revealed {
            icons::BWI_EYE_SLASH
        } else {
            icons::BWI_EYE
        }
        .render(18.0, self.colors.text_primary);

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

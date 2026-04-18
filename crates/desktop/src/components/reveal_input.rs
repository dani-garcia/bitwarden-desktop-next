//! Password-style input with a self-managed reveal toggle.
//!
//! Mirrors the look of [`crate::components::inputs::floating_label_input`]
//! with a show/hide eye button — but crucially, the `revealed: bool` state
//! lives inside the widget tree (via iced's `Component` + `tree::State`),
//! so callers don't need to thread `*_visible` flags and toggle messages
//! through their own state. Same ergonomic shape as `pick_list`: you pass
//! the value, you get edits back, everything else is managed.
//!
//! ## Why `Component` (deprecated)?
//!
//! iced's `Component` trait carries a deprecation notice ("introduces
//! encapsulated state"), but that deprecation is philosophical — the
//! replacement is to implement `Widget` manually. For a compound of
//! `text_input` + a button, that means reimplementing the bridge we get
//! here for free. Component is the right tool for widgets that genuinely
//! *should* own their transient state (e.g. whether the password is
//! currently visible has no meaning outside the widget). If the feature
//! is ever removed upstream we can swap in a hand-rolled Widget.

#![allow(deprecated)] // `Component` is deprecation-tagged upstream; see module docs.

use iced::{
    Alignment, Element,
    widget::{Component, component, row},
};

use crate::{
    components::{
        buttons, icons,
        inputs::{bare_text_input, floating_label_frame},
    },
    theme::{AppColors, AppTheme},
};

/// Build a password/hidden-value input with self-managed reveal state.
///
/// `label` is the floating chip. `value` is the current text. `on_input` is
/// fired on every keystroke. `disabled=true` makes the text input read-only
/// and the toggle inert.
pub fn reveal_input<'a, Message>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, Message, AppTheme>
where
    Message: Clone + 'a,
{
    component(RevealInput {
        label,
        value,
        on_input: Box::new(on_input),
        on_submit: None,
        disabled,
        colors,
    })
}

/// Variant of [`reveal_input`] that also fires `on_submit` when the user
/// presses Enter inside the field.
pub fn reveal_input_with_submit<'a, Message>(
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
    component(RevealInput {
        label,
        value,
        on_input: Box::new(on_input),
        on_submit: Some(on_submit),
        disabled,
        colors,
    })
}

struct RevealInput<'a, Message> {
    label: &'a str,
    value: &'a str,
    on_input: Box<dyn Fn(String) -> Message + 'a>,
    on_submit: Option<Message>,
    disabled: bool,
    colors: &'a AppColors,
}

#[derive(Default)]
struct State {
    revealed: bool,
}

#[derive(Clone)]
enum Event {
    Input(String),
    Submit,
    Toggle,
}

impl<'a, Message: Clone + 'a> Component<Message, AppTheme> for RevealInput<'a, Message> {
    type State = State;
    type Event = Event;

    fn update(&mut self, state: &mut State, event: Event) -> Option<Message> {
        match event {
            Event::Input(s) => Some((self.on_input)(s)),
            Event::Submit => self.on_submit.clone(),
            Event::Toggle => {
                state.revealed = !state.revealed;
                None
            }
        }
    }

    fn view(&self, state: &State) -> Element<'_, Event, AppTheme> {
        let mut input = bare_text_input(self.value);
        if !self.disabled {
            input = input.on_input(Event::Input);
            if self.on_submit.is_some() {
                input = input.on_submit(Event::Submit);
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
            toggle_button = toggle_button.on_press(Event::Toggle);
        }

        let input_row = row![input, toggle_button].align_y(Alignment::Center);
        floating_label_frame(self.label, input_row.into(), self.colors)
    }
}

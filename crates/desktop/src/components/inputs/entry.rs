//! Editable text fields: the `text_field` builder and `stepper_field`.

use iced::{
    Alignment, Color, Element,
    widget::{column, row},
};

use crate::{
    components::{buttons, icons},
    theme::{AppColors, AppTheme},
};

use super::{
    bare_text_input, errored_field_frame, errored_field_frame_on, field_frame, field_frame_on,
};

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
        match (errored, chip_bg) {
            (true, Some(chip_bg)) => {
                errored_field_frame_on(label, content, move |c| chip_bg(c), colors)
            }
            (true, None) => errored_field_frame(label, content, colors),
            (false, Some(chip_bg)) => field_frame_on(label, content, move |c| chip_bg(c), colors),
            (false, None) => field_frame(label, content, colors),
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

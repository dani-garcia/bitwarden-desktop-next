//! Single-select picker (`select_field`), searchable single-select
//! (`search_select_field`), and multi-select panel (`multi_select_field`).

use std::fmt::Display;

use iced::{
    Background, Border, Color, Element, Fill,
    widget::{Component, combo_box, component, pick_list, text_input},
};

use crate::{
    components::{drop_down::DropDown, icons},
    theme::{AppColors, AppTheme},
};

use super::{field_frame, field_frame_on};

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

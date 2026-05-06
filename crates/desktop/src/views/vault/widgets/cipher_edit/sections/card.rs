//! Card-type section + the brand / exp-month selectors that only apply here.

use iced::{Element, widget::column};

use crate::{
    components::inputs::{reveal_text_field, select_field, text_field},
    fl,
    theme::{AppColors, AppTheme},
    views::vault::widgets::{
        cipher_edit::{CipherEditMessage, CipherForm},
        field_helpers::{card_with_margin, styled_card},
    },
};

pub(in super::super) fn card_details_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let Some(c) = form.modified.card.as_ref() else {
        return iced::widget::Space::new().into();
    };

    let body = column![
        text_field(
            fl!("form-card-cardholder"),
            c.cardholder_name.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::CardCardholderChanged)
        .disabled(form.saving),
        brand_selector(form, colors),
        text_field(
            fl!("form-card-number"),
            c.number.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::CardNumberChanged)
        .disabled(form.saving),
        exp_month_selector(form, colors),
        text_field(
            fl!("form-card-exp-year"),
            c.exp_year.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::CardExpYearChanged)
        .disabled(form.saving),
        reveal_text_field(
            fl!("form-card-code"),
            c.code.as_deref().unwrap_or(""),
            CipherEditMessage::CardCodeChanged,
            form.saving,
            colors,
        ),
    ]
    .spacing(12);

    card_with_margin(styled_card(body))
}

fn brand_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut options: Vec<Option<String>> = vec![None];
    options.extend(
        [
            "Visa",
            "Mastercard",
            "Amex",
            "Discover",
            "Diners Club",
            "JCB",
            "Maestro",
            "UnionPay",
            "RuPay",
            "Other",
        ]
        .into_iter()
        .map(|b| Some(b.to_string())),
    );

    let selected = form.modified.card.as_ref().map(|c| c.brand.clone());

    select_field(
        fl!("form-card-brand"),
        selected,
        options,
        |choice: &Option<String>| match choice {
            None => fl!("form-card-brand-placeholder"),
            Some(s) => s.clone(),
        },
        CipherEditMessage::CardBrandSelected,
        colors,
    )
}

fn exp_month_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut options: Vec<Option<String>> = vec![None];
    options.extend((1..=12).map(|m| Some(format!("{m:02}"))));

    let selected = form.modified.card.as_ref().map(|c| c.exp_month.clone());

    select_field(
        fl!("form-card-exp-month"),
        selected,
        options,
        |choice: &Option<String>| match choice {
            None => fl!("form-card-month-placeholder"),
            Some(s) => s.clone(),
        },
        CipherEditMessage::CardExpMonthSelected,
        colors,
    )
}

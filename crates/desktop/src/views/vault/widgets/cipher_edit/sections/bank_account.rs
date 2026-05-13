//! Bank-account-type section + the account-type selector.

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

pub(in super::super) fn bank_account_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let Some(b) = form.modified.bank_account.as_ref() else {
        return iced::widget::Space::new().into();
    };

    let body = column![
        text_field(
            fl!("form-bank-name"),
            b.bank_name.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankNameChanged)
        .disabled(form.saving),
        text_field(
            fl!("form-bank-name-on-account"),
            b.name_on_account.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankNameOnAccountChanged)
        .disabled(form.saving),
        account_type_selector(form, colors),
        reveal_text_field(
            fl!("form-bank-account-number"),
            b.account_number.as_deref().unwrap_or(""),
            CipherEditMessage::BankAccountNumberChanged,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-bank-routing-number"),
            b.routing_number.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankRoutingNumberChanged)
        .disabled(form.saving),
        text_field(
            fl!("form-bank-branch-number"),
            b.branch_number.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankBranchNumberChanged)
        .disabled(form.saving),
        reveal_text_field(
            fl!("form-bank-pin"),
            b.pin.as_deref().unwrap_or(""),
            CipherEditMessage::BankPinChanged,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-bank-swift-code"),
            b.swift_code.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankSwiftCodeChanged)
        .disabled(form.saving),
        text_field(
            fl!("form-bank-iban"),
            b.iban.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankIbanChanged)
        .disabled(form.saving),
        text_field(
            fl!("form-bank-contact-phone"),
            b.bank_contact_phone.as_deref().unwrap_or(""),
            colors,
        )
        .on_input(CipherEditMessage::BankContactPhoneChanged)
        .disabled(form.saving),
    ]
    .spacing(12);

    card_with_margin(styled_card(body))
}

fn account_type_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherEditMessage, AppTheme> {
    let mut options: Vec<Option<String>> = vec![None];
    options.extend(BANK_ACCOUNT_TYPES.iter().map(|t| Some((*t).to_string())));

    let selected = form
        .modified
        .bank_account
        .as_ref()
        .map(|b| b.account_type.clone());

    select_field(
        fl!("form-bank-account-type"),
        selected,
        options,
        |choice: &Option<String>| match choice {
            None => fl!("form-bank-account-type-placeholder"),
            Some(s) => bank_account_type_label(s),
        },
        CipherEditMessage::BankAccountTypeSelected,
        colors,
    )
}

/// Canonical bank-account type values, stored on `BankAccountView::account_type`
/// as-is. Localized for display via [`bank_account_type_label`]; the stored
/// value is always one of these English strings so sync with other clients
/// produces matching subtitles.
const BANK_ACCOUNT_TYPES: &[&str] = &[
    "Checking",
    "Savings",
    "Brokerage",
    "Money market",
    "Certificate of deposit (CD)",
    "Other",
];

fn bank_account_type_label(value: &str) -> String {
    match value {
        "Checking" => fl!("form-bank-account-type-checking"),
        "Savings" => fl!("form-bank-account-type-savings"),
        "Brokerage" => fl!("form-bank-account-type-brokerage"),
        "Money market" => fl!("form-bank-account-type-money-market"),
        "Certificate of deposit (CD)" => fl!("form-bank-account-type-cd"),
        "Other" => fl!("form-bank-account-type-other"),
        other => other.to_owned(),
    }
}

//! Identity-type section cards (four: personal, identification, contact,
//! address) plus the title selector and its localization map.

use iced::{Element, widget::column};

use crate::{
    components::inputs::{reveal_text_field, select_field, text_field},
    fl,
    theme::{AppColors, AppTheme},
};

use super::super::super::field_helpers::{card_with_margin, styled_card};
use super::super::{message::CipherFormMessage, state::CipherForm};

pub(in super::super) fn identity_personal_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        title_selector(form, colors),
        text_field(
            fl!("form-identity-first-name"),
            i.first_name.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityFirstNameChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-middle-name"),
            i.middle_name.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityMiddleNameChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-last-name"),
            i.last_name.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityLastNameChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-username"),
            i.username.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityUsernameChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-company"),
            i.company.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityCompanyChanged,
            None,
            form.saving,
            colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

pub(in super::super) fn identity_identification_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        reveal_text_field(
            fl!("form-identity-ssn"),
            i.ssn.as_deref().unwrap_or(""),
            CipherFormMessage::IdentitySsnChanged,
            form.saving,
            colors,
        ),
        reveal_text_field(
            fl!("form-identity-passport"),
            i.passport_number.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityPassportChanged,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-license"),
            i.license_number.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityLicenseChanged,
            None,
            form.saving,
            colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

pub(in super::super) fn identity_contact_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        text_field(
            fl!("form-identity-email"),
            i.email.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityEmailChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-phone"),
            i.phone.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityPhoneChanged,
            None,
            form.saving,
            colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

pub(in super::super) fn identity_address_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        text_field(
            fl!("form-identity-address1"),
            i.address1.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityAddress1Changed,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-address2"),
            i.address2.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityAddress2Changed,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-address3"),
            i.address3.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityAddress3Changed,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-city"),
            i.city.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityCityChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-state"),
            i.state.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityStateChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-postal"),
            i.postal_code.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityPostalCodeChanged,
            None,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-identity-country"),
            i.country.as_deref().unwrap_or(""),
            CipherFormMessage::IdentityCountryChanged,
            None,
            form.saving,
            colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn title_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let mut options: Vec<Option<String>> = vec![None];
    options.extend(IDENTITY_TITLES.iter().map(|t| Some((*t).to_string())));

    let selected = form.modified.identity.as_ref().map(|i| i.title.clone());

    select_field(
        fl!("form-identity-title"),
        selected,
        options,
        |choice: &Option<String>| match choice {
            None => fl!("form-identity-title-placeholder"),
            Some(s) => identity_title_label(s),
        },
        CipherFormMessage::IdentityTitleSelected,
        colors,
    )
}

/// Canonical identity title values, stored on `IdentityView::title` as-is.
/// Localized for display via [`identity_title_label`]; the stored value is
/// always one of these English strings so sync with other clients matches.
const IDENTITY_TITLES: &[&str] = &["Mr", "Mrs", "Ms", "Mx", "Dr"];

fn identity_title_label(value: &str) -> String {
    match value {
        "Mr" => fl!("form-identity-title-mr"),
        "Mrs" => fl!("form-identity-title-mrs"),
        "Ms" => fl!("form-identity-title-ms"),
        "Mx" => fl!("form-identity-title-mx"),
        "Dr" => fl!("form-identity-title-dr"),
        other => other.to_owned(),
    }
}

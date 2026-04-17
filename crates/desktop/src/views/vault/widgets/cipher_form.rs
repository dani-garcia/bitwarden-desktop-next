//! Editable cipher form — the flip-side of `detail_pane`.
//!
//! Layout mirrors the Angular `add-edit-v2` component so field coverage and
//! grouping match the official clients. Same section primitives as
//! `detail_pane` (via `field_helpers`) so view and edit modes share a
//! consistent visual rhythm.
//!
//! `CipherForm` holds two `CipherView`s: an untouched `original` (for cancel
//! and diffing) and a `modified` that form events mutate in place. Save hands
//! `modified` back to `ClientManager::save_cipher` which encrypts and
//! persists via the per-user SQLite repo.

use std::collections::HashMap;

use bitwarden_collections::collection::CollectionId;
use bitwarden_core::OrganizationId;
use bitwarden_vault::{
    CardView, CipherRepromptType, CipherType, CipherView, FieldType, FieldView, FolderId,
    FolderView, IdentityView, LoginUriView, LoginView, SecureNoteType, SecureNoteView, SshKeyView,
    UriMatchType,
};
use iced::{
    Alignment, Background, Border, Color, Element, Fill,
    widget::{Space, checkbox, column, container, row, scrollable, text, text_input},
};

use super::field_helpers::{card_with_margin, field_readonly, section_label, styled_card};
use crate::{
    components::{
        self, buttons,
        drop_down::{self, DropDown},
        icons,
    },
    sdk::{Collection, Organization},
    theme::{AppColors, AppTheme, RADIUS_SM},
};

// ── Public state ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct FolderOption {
    pub id: FolderId,
    pub name: String,
}

impl From<FolderView> for FolderOption {
    fn from(v: FolderView) -> Self {
        Self {
            // Folders from `FoldersClient::list()` always have an id populated
            // by the SDK — we require it here so dropdown selection doesn't
            // have to cope with `Option<FolderId>` per entry.
            id: v.id.expect("decrypted folder missing id"),
            name: v.name,
        }
    }
}

pub type OrganizationOption = Organization;
pub type CollectionOption = Collection;

/// Form backing the cipher edit view. Also sized to back the "New item"
/// flow (`original = None`), though wiring that is out of scope here.
pub struct CipherForm {
    pub original: Option<CipherView>,
    pub modified: CipherView,

    pub folders: Vec<FolderOption>,
    pub organizations: Vec<OrganizationOption>,
    pub collections: Vec<CollectionOption>,

    // Per-field reveal flags
    pub password_visible: bool,
    pub card_code_visible: bool,
    pub identity_ssn_visible: bool,
    pub identity_passport_visible: bool,
    pub custom_field_reveals: HashMap<usize, bool>,

    // Dropdown open flags
    pub folder_dropdown_open: bool,
    pub org_dropdown_open: bool,
    pub collections_dropdown_open: bool,
    pub card_brand_dropdown_open: bool,
    pub card_exp_month_dropdown_open: bool,
    pub identity_title_dropdown_open: bool,
    /// Custom-field type dropdowns keyed by field index.
    pub custom_field_type_dropdowns: HashMap<usize, bool>,

    /// Disables the Save button + form inputs while the save task is in flight.
    pub saving: bool,
}

impl CipherForm {
    /// Construct for editing an existing cipher. Clones the view so `original`
    /// stays pristine regardless of what form events do to `modified`.
    pub fn edit(cv: CipherView) -> Self {
        let mut form = Self {
            original: Some(cv.clone()),
            modified: cv,
            folders: Vec::new(),
            organizations: Vec::new(),
            collections: Vec::new(),
            password_visible: false,
            card_code_visible: false,
            identity_ssn_visible: false,
            identity_passport_visible: false,
            custom_field_reveals: HashMap::new(),
            folder_dropdown_open: false,
            org_dropdown_open: false,
            collections_dropdown_open: false,
            card_brand_dropdown_open: false,
            card_exp_month_dropdown_open: false,
            identity_title_dropdown_open: false,
            custom_field_type_dropdowns: HashMap::new(),
            saving: false,
        };
        form.ensure_sub_structs();
        form
    }

    /// Ensure the type-specific sub-view (`login`, `card`, ...) is populated
    /// on `modified` so form handlers can mutate `.as_mut()?` without None
    /// short-circuiting on first edit.
    fn ensure_sub_structs(&mut self) {
        match self.modified.r#type {
            CipherType::Login => {
                self.modified.login.get_or_insert(LoginView {
                    username: None,
                    password: None,
                    password_revision_date: None,
                    uris: None,
                    totp: None,
                    autofill_on_page_load: None,
                    fido2_credentials: None,
                });
            }
            CipherType::Card => {
                self.modified.card.get_or_insert(CardView {
                    cardholder_name: None,
                    exp_month: None,
                    exp_year: None,
                    code: None,
                    brand: None,
                    number: None,
                });
            }
            CipherType::Identity => {
                self.modified.identity.get_or_insert(IdentityView {
                    title: None,
                    first_name: None,
                    middle_name: None,
                    last_name: None,
                    address1: None,
                    address2: None,
                    address3: None,
                    city: None,
                    state: None,
                    postal_code: None,
                    country: None,
                    company: None,
                    email: None,
                    phone: None,
                    ssn: None,
                    username: None,
                    passport_number: None,
                    license_number: None,
                });
            }
            CipherType::SecureNote => {
                self.modified.secure_note.get_or_insert(SecureNoteView {
                    r#type: SecureNoteType::Generic,
                });
            }
            CipherType::SshKey => { /* SSH key fields read-only for now */ }
        }
    }

    /// Router-triggered close helper; flips any open dropdown closed.
    pub fn dismiss_dropdowns(&mut self) {
        self.folder_dropdown_open = false;
        self.org_dropdown_open = false;
        self.collections_dropdown_open = false;
        self.card_brand_dropdown_open = false;
        self.card_exp_month_dropdown_open = false;
        self.identity_title_dropdown_open = false;
        self.custom_field_type_dropdowns.clear();
    }
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum CipherFormMessage {
    // Item details (universal)
    NameChanged(String),
    NotesChanged(String),
    FavoriteToggled,
    RepromptToggled,

    // Ownership
    FolderDropdownToggled,
    FolderSelected(Option<FolderId>),
    OrgDropdownToggled,
    OrgSelected(Option<OrganizationId>),
    CollectionsDropdownToggled,
    CollectionToggled(CollectionId),

    // Login
    UsernameChanged(String),
    PasswordChanged(String),
    TotpChanged(String),
    TogglePasswordVisibility,
    UriChanged(usize, String),
    UriAdded,
    UriRemoved(usize),

    // Card
    CardCardholderChanged(String),
    CardBrandDropdownToggled,
    CardBrandSelected(Option<String>),
    CardNumberChanged(String),
    CardExpMonthDropdownToggled,
    CardExpMonthSelected(Option<String>),
    CardExpYearChanged(String),
    CardCodeChanged(String),
    ToggleCardCodeVisibility,

    // Identity
    IdentityTitleDropdownToggled,
    IdentityTitleSelected(Option<String>),
    IdentityFirstNameChanged(String),
    IdentityMiddleNameChanged(String),
    IdentityLastNameChanged(String),
    IdentityUsernameChanged(String),
    IdentityCompanyChanged(String),
    IdentitySsnChanged(String),
    IdentityPassportChanged(String),
    IdentityLicenseChanged(String),
    ToggleIdentitySsnVisibility,
    ToggleIdentityPassportVisibility,
    IdentityEmailChanged(String),
    IdentityPhoneChanged(String),
    IdentityAddress1Changed(String),
    IdentityAddress2Changed(String),
    IdentityAddress3Changed(String),
    IdentityCityChanged(String),
    IdentityStateChanged(String),
    IdentityPostalCodeChanged(String),
    IdentityCountryChanged(String),

    // Custom fields
    CustomFieldAdded,
    CustomFieldRemoved(usize),
    CustomFieldTypeDropdownToggled(usize),
    CustomFieldTypeSelected(usize, FieldType),
    CustomFieldNameChanged(usize, String),
    CustomFieldValueChanged(usize, String),
    CustomFieldBoolToggled(usize),
    ToggleCustomFieldReveal(usize),

    // Flow
    Save,
    Cancel,
}

// ── Update ─────────────────────────────────────────────────────────────────

/// Result of applying a form message. Most messages only mutate the form;
/// Save/Cancel bubble up so the vault router can kick off the save task or
/// dispose the form.
pub enum FormAction {
    None,
    Save,
    Cancel,
}

impl CipherForm {
    pub fn update(&mut self, msg: CipherFormMessage) -> FormAction {
        use CipherFormMessage::*;
        match msg {
            // Item details
            NameChanged(s) => self.modified.name = s,
            NotesChanged(s) => self.modified.notes = opt_string(s),
            FavoriteToggled => self.modified.favorite = !self.modified.favorite,
            RepromptToggled => {
                self.modified.reprompt = match self.modified.reprompt {
                    CipherRepromptType::None => CipherRepromptType::Password,
                    CipherRepromptType::Password => CipherRepromptType::None,
                };
            }

            // Ownership
            FolderDropdownToggled => {
                self.folder_dropdown_open = !self.folder_dropdown_open;
            }
            FolderSelected(id) => {
                self.modified.folder_id = id;
                self.folder_dropdown_open = false;
            }
            OrgDropdownToggled => {
                self.org_dropdown_open = !self.org_dropdown_open;
            }
            OrgSelected(id) => {
                self.modified.organization_id = id;
                // Clearing the org clears any collections that were scoped to it.
                if id.is_none() {
                    self.modified.collection_ids.clear();
                } else {
                    self.modified
                        .collection_ids
                        .retain(|cid| self.collections.iter().any(|c| &c.id == cid && Some(c.organization_id) == id));
                }
                self.org_dropdown_open = false;
            }
            CollectionsDropdownToggled => {
                self.collections_dropdown_open = !self.collections_dropdown_open;
            }
            CollectionToggled(cid) => {
                if let Some(pos) = self.modified.collection_ids.iter().position(|c| c == &cid) {
                    self.modified.collection_ids.remove(pos);
                } else {
                    self.modified.collection_ids.push(cid);
                }
            }

            // Login
            UsernameChanged(s) => {
                if let Some(l) = self.modified.login.as_mut() {
                    l.username = opt_string(s);
                }
            }
            PasswordChanged(s) => {
                if let Some(l) = self.modified.login.as_mut() {
                    l.password = opt_string(s);
                }
            }
            TotpChanged(s) => {
                if let Some(l) = self.modified.login.as_mut() {
                    l.totp = opt_string(s);
                }
            }
            TogglePasswordVisibility => self.password_visible = !self.password_visible,
            UriChanged(idx, s) => {
                if let Some(l) = self.modified.login.as_mut() {
                    let uris = l.uris.get_or_insert_default();
                    if let Some(u) = uris.get_mut(idx) {
                        u.uri = opt_string(s);
                    }
                }
            }
            UriAdded => {
                if let Some(l) = self.modified.login.as_mut() {
                    l.uris.get_or_insert_default().push(LoginUriView {
                        uri: Some(String::new()),
                        r#match: Some(UriMatchType::Domain),
                        uri_checksum: None,
                    });
                }
            }
            UriRemoved(idx) => {
                if let Some(l) = self.modified.login.as_mut()
                    && let Some(uris) = l.uris.as_mut()
                    && idx < uris.len()
                {
                    uris.remove(idx);
                }
            }

            // Card
            CardCardholderChanged(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.cardholder_name = opt_string(s);
                }
            }
            CardBrandDropdownToggled => {
                self.card_brand_dropdown_open = !self.card_brand_dropdown_open;
            }
            CardBrandSelected(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.brand = s;
                }
                self.card_brand_dropdown_open = false;
            }
            CardNumberChanged(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.number = opt_string(s);
                }
            }
            CardExpMonthDropdownToggled => {
                self.card_exp_month_dropdown_open = !self.card_exp_month_dropdown_open;
            }
            CardExpMonthSelected(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.exp_month = s;
                }
                self.card_exp_month_dropdown_open = false;
            }
            CardExpYearChanged(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.exp_year = opt_string(s);
                }
            }
            CardCodeChanged(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.code = opt_string(s);
                }
            }
            ToggleCardCodeVisibility => self.card_code_visible = !self.card_code_visible,

            // Identity
            IdentityTitleDropdownToggled => {
                self.identity_title_dropdown_open = !self.identity_title_dropdown_open;
            }
            IdentityTitleSelected(s) => {
                if let Some(i) = self.modified.identity.as_mut() {
                    i.title = s;
                }
                self.identity_title_dropdown_open = false;
            }
            IdentityFirstNameChanged(s) => identity_set(&mut self.modified, |i| &mut i.first_name, s),
            IdentityMiddleNameChanged(s) => identity_set(&mut self.modified, |i| &mut i.middle_name, s),
            IdentityLastNameChanged(s) => identity_set(&mut self.modified, |i| &mut i.last_name, s),
            IdentityUsernameChanged(s) => identity_set(&mut self.modified, |i| &mut i.username, s),
            IdentityCompanyChanged(s) => identity_set(&mut self.modified, |i| &mut i.company, s),
            IdentitySsnChanged(s) => identity_set(&mut self.modified, |i| &mut i.ssn, s),
            IdentityPassportChanged(s) => identity_set(&mut self.modified, |i| &mut i.passport_number, s),
            IdentityLicenseChanged(s) => identity_set(&mut self.modified, |i| &mut i.license_number, s),
            ToggleIdentitySsnVisibility => self.identity_ssn_visible = !self.identity_ssn_visible,
            ToggleIdentityPassportVisibility => {
                self.identity_passport_visible = !self.identity_passport_visible;
            }
            IdentityEmailChanged(s) => identity_set(&mut self.modified, |i| &mut i.email, s),
            IdentityPhoneChanged(s) => identity_set(&mut self.modified, |i| &mut i.phone, s),
            IdentityAddress1Changed(s) => identity_set(&mut self.modified, |i| &mut i.address1, s),
            IdentityAddress2Changed(s) => identity_set(&mut self.modified, |i| &mut i.address2, s),
            IdentityAddress3Changed(s) => identity_set(&mut self.modified, |i| &mut i.address3, s),
            IdentityCityChanged(s) => identity_set(&mut self.modified, |i| &mut i.city, s),
            IdentityStateChanged(s) => identity_set(&mut self.modified, |i| &mut i.state, s),
            IdentityPostalCodeChanged(s) => identity_set(&mut self.modified, |i| &mut i.postal_code, s),
            IdentityCountryChanged(s) => identity_set(&mut self.modified, |i| &mut i.country, s),

            // Custom fields
            CustomFieldAdded => {
                let fields = self.modified.fields.get_or_insert_default();
                fields.push(FieldView {
                    name: Some(String::new()),
                    value: Some(String::new()),
                    r#type: FieldType::Text,
                    linked_id: None,
                });
            }
            CustomFieldRemoved(idx) => {
                if let Some(fields) = self.modified.fields.as_mut()
                    && idx < fields.len()
                {
                    fields.remove(idx);
                    self.custom_field_reveals.remove(&idx);
                    self.custom_field_type_dropdowns.remove(&idx);
                }
            }
            CustomFieldTypeDropdownToggled(idx) => {
                let entry = self
                    .custom_field_type_dropdowns
                    .entry(idx)
                    .or_insert(false);
                *entry = !*entry;
            }
            CustomFieldTypeSelected(idx, ty) => {
                if let Some(fields) = self.modified.fields.as_mut()
                    && let Some(f) = fields.get_mut(idx)
                {
                    f.r#type = ty;
                    // Boolean fields represent value as "true"/"false" strings;
                    // reset to a known default when changing to/from bool so the
                    // widget rendering doesn't try to parse a free-form string.
                    if matches!(ty, FieldType::Boolean)
                        && !matches!(f.value.as_deref(), Some("true") | Some("false"))
                    {
                        f.value = Some("false".to_string());
                    }
                }
                self.custom_field_type_dropdowns.insert(idx, false);
            }
            CustomFieldNameChanged(idx, s) => {
                if let Some(fields) = self.modified.fields.as_mut()
                    && let Some(f) = fields.get_mut(idx)
                {
                    f.name = opt_string(s);
                }
            }
            CustomFieldValueChanged(idx, s) => {
                if let Some(fields) = self.modified.fields.as_mut()
                    && let Some(f) = fields.get_mut(idx)
                {
                    f.value = opt_string(s);
                }
            }
            CustomFieldBoolToggled(idx) => {
                if let Some(fields) = self.modified.fields.as_mut()
                    && let Some(f) = fields.get_mut(idx)
                {
                    let next = !matches!(f.value.as_deref(), Some("true"));
                    f.value = Some(if next { "true" } else { "false" }.to_string());
                }
            }
            ToggleCustomFieldReveal(idx) => {
                let entry = self.custom_field_reveals.entry(idx).or_insert(false);
                *entry = !*entry;
            }

            // Flow
            Save => return FormAction::Save,
            Cancel => return FormAction::Cancel,
        }
        FormAction::None
    }
}

fn opt_string(s: String) -> Option<String> {
    if s.is_empty() { None } else { Some(s) }
}

// ── Input primitives ───────────────────────────────────────────────────────
//
// The form uses a plain "label above input" layout (column) rather than the
// floating-label `stack` pattern from the login view. `stack` lays out both
// children at the same bounds on every frame (see CLAUDE.md → "Stack doesn't
// cull or clip") — at ~20 inputs in the identity form that's a measurable
// extra cost per scroll frame. Column is strictly cheaper and reads fine in
// a form context where every field has a label.

fn labeled_input<'a, M>(
    label: &'a str,
    value: &'a str,
    on_input: impl Fn(String) -> M + 'a,
    secure: bool,
    show_toggle: Option<(bool, M)>,
    disabled: bool,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme>
where
    M: Clone + 'a,
{
    let mut input = text_input("", value)
        .size(14)
        .padding([8, 12])
        .width(Fill)
        .style(|theme: &AppTheme, _status| text_input::Style {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(RADIUS_SM),
            icon: theme.colors.text_muted,
            placeholder: theme.colors.text_secondary,
            value: theme.colors.text_primary,
            selection: theme.colors.accent,
        });
    if !disabled {
        input = input.on_input(on_input);
    }
    if secure {
        input = input.secure(true);
    }

    let control: Element<'a, M, AppTheme> = if let Some((is_visible, toggle_msg)) = show_toggle {
        let toggle_icon = if is_visible {
            icons::EYE
        } else {
            icons::EYE_SLASH
        }
        .render(16.0, colors.text_secondary);

        let mut toggle_button =
            buttons::ghost_icon(toggle_icon, colors.item_hover).padding([8, 10]);
        if !disabled {
            toggle_button = toggle_button.on_press(toggle_msg);
        }
        row![input, toggle_button].align_y(Alignment::Center).into()
    } else {
        input.into()
    };

    column![
        text(label).size(12).color(colors.text_muted),
        control,
    ]
    .spacing(4)
    .into()
}

fn identity_set(cv: &mut CipherView, pick: impl Fn(&mut IdentityView) -> &mut Option<String>, s: String) {
    if let Some(i) = cv.identity.as_mut() {
        *pick(i) = opt_string(s);
    }
}

// ── View ───────────────────────────────────────────────────────────────────

pub fn view<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
    top_radius: f32,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let header = header_row(form, colors);

    let mut sections: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        section_label("Item details", colors),
        item_details_card(form, colors),
    ];

    match form.modified.r#type {
        CipherType::Login => {
            sections.push(section_label("Login credentials", colors));
            sections.push(login_card(form, colors));
            sections.push(section_label("Autofill options", colors));
            sections.push(autofill_card(form, colors));
        }
        CipherType::Card => {
            sections.push(section_label("Card details", colors));
            sections.push(card_details_card(form, colors));
        }
        CipherType::Identity => {
            sections.push(section_label("Personal details", colors));
            sections.push(identity_personal_card(form, colors));
            sections.push(section_label("Identification", colors));
            sections.push(identity_identification_card(form, colors));
            sections.push(section_label("Contact info", colors));
            sections.push(identity_contact_card(form, colors));
            sections.push(section_label("Address", colors));
            sections.push(identity_address_card(form, colors));
        }
        CipherType::SecureNote => { /* notes live in the shared "Additional options" card below */ }
        CipherType::SshKey => {
            sections.push(section_label("SSH key", colors));
            sections.push(ssh_key_card(form, colors));
        }
    }

    sections.push(section_label("Additional options", colors));
    sections.push(additional_options_card(form, colors));

    sections.push(section_label("Custom fields", colors));
    sections.push(custom_fields_card(form, colors));

    let body = scrollable(column(sections).spacing(4).padding([12, 20])).height(Fill);
    let bottom_bar = bottom_bar(form, colors);

    container(column![header, body, bottom_bar].spacing(0).height(Fill))
        .width(Fill)
        .height(Fill)
        .style(move |theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    iced::Border::default()
                        .rounded(iced::border::top_left(top_radius).top_right(top_radius)),
                )
        })
        .into()
}

// ── Header & bottom bar ────────────────────────────────────────────────────

fn header_row<'a>(
    form: &'a CipherForm,
    colors: &AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let title_text = if form.original.is_some() {
        match form.modified.r#type {
            CipherType::Login => "Edit login",
            CipherType::Card => "Edit card",
            CipherType::Identity => "Edit identity",
            CipherType::SecureNote => "Edit note",
            CipherType::SshKey => "Edit SSH key",
        }
    } else {
        "New item"
    };

    let title = text(title_text).size(18).color(colors.text_primary);

    let cancel_btn = buttons::ghost_icon(
        icons::BWI_CLOSE.render(32.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(CipherFormMessage::Cancel)
    .padding([1, 1]);

    let header = container(
        row![title, Space::new().width(Fill), cancel_btn].align_y(Alignment::Center),
    )
    .padding([8, 20]);

    column![header, components::separator_h()].spacing(0).into()
}

fn bottom_bar<'a>(
    form: &'a CipherForm,
    _colors: &AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let save_label = if form.saving { "Saving…" } else { "Save" };
    let mut save_btn = buttons::primary(text(save_label).size(14)).padding([8, 20]);
    if !form.saving {
        save_btn = save_btn.on_press(CipherFormMessage::Save);
    }

    let cancel_btn = buttons::secondary(text("Cancel").size(14))
        .on_press(CipherFormMessage::Cancel)
        .padding([8, 20]);

    let bar = container(
        row![save_btn, cancel_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .padding([8, 20])
    .style(|theme: &AppTheme| {
        container::Style::default().background(theme.colors.background)
    });

    column![components::separator_h(), bar].spacing(0).into()
}

// ── Sections ───────────────────────────────────────────────────────────────

fn item_details_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();

    rows.push(labeled_input("Name (required)", &form.modified.name, CipherFormMessage::NameChanged, false, None, form.saving, colors,
    ));

    // Favorite + reprompt toggles in a row so the card stays compact.
    let favorite_checkbox = checkbox(form.modified.favorite)
        .label("Favorite")
        .on_toggle(|_| CipherFormMessage::FavoriteToggled)
        .size(18)
        .spacing(8);

    rows.push(favorite_checkbox.into());

    // Folder dropdown (personal vault only — orgs own their own folder concept)
    rows.push(folder_selector(form, colors));

    // Organization dropdown (if user belongs to any orgs)
    if !form.organizations.is_empty() {
        rows.push(org_selector(form, colors));

        if form.modified.organization_id.is_some() {
            rows.push(collections_selector(form, colors));
        }
    }

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn login_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let login = form.modified.login.as_ref().expect("ensure_sub_structs");

    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        labeled_input("Username", login.username.as_deref().unwrap_or(""), CipherFormMessage::UsernameChanged, false, None, form.saving, colors,
        ),
        labeled_input("Password", login.password.as_deref().unwrap_or(""), CipherFormMessage::PasswordChanged, !form.password_visible, Some((
                form.password_visible, CipherFormMessage::TogglePasswordVisibility,  )),
            form.saving,
            colors,
        ),
        labeled_input("Authenticator key (TOTP)", login.totp.as_deref().unwrap_or(""), CipherFormMessage::TotpChanged, false, None, form.saving, colors,
        ),
    ];

    card_with_margin(styled_card(column(rows).spacing(16).into()))
}

fn autofill_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();

    let uris = form
        .modified
        .login
        .as_ref()
        .and_then(|l| l.uris.as_deref())
        .unwrap_or(&[]);

    if uris.is_empty() {
        rows.push(
            text("No websites yet")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    } else {
        for (idx, uri) in uris.iter().enumerate() {
            let value = uri.uri.as_deref().unwrap_or("");
            let input = labeled_input(
                "Website (URI)",
                value,
                move |s| CipherFormMessage::UriChanged(idx, s),
                false,
                None,
                form.saving,
                colors,
            );
            let remove_btn = buttons::ghost_icon(
                icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
                colors.item_hover,
            )
            .on_press(CipherFormMessage::UriRemoved(idx))
            .padding([6, 6]);

            rows.push(
                row![
                    container(input).width(Fill),
                    remove_btn,
                ]
                .spacing(6)
                .align_y(Alignment::Center)
                .into(),
            );
        }
    }

    let add_btn = buttons::secondary(
        row![
            icons::PLUS.render(14.0, colors.accent),
            text("Add website").size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(CipherFormMessage::UriAdded)
    .padding([6, 12]);
    rows.push(add_btn.into());

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn card_details_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let c = form.modified.card.as_ref().expect("ensure_sub_structs");

    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        labeled_input("Cardholder name", c.cardholder_name.as_deref().unwrap_or(""), CipherFormMessage::CardCardholderChanged, false, None, form.saving, colors,
        ),
        brand_selector(form, colors),
        labeled_input("Number", c.number.as_deref().unwrap_or(""), CipherFormMessage::CardNumberChanged, false, None, form.saving, colors,
        ),
        exp_month_selector(form, colors),
        labeled_input("Expiration year", c.exp_year.as_deref().unwrap_or(""), CipherFormMessage::CardExpYearChanged, false, None, form.saving, colors,
        ),
        labeled_input("Security code", c.code.as_deref().unwrap_or(""), CipherFormMessage::CardCodeChanged, !form.card_code_visible, Some((form.card_code_visible, CipherFormMessage::ToggleCardCodeVisibility)), form.saving,
            colors,
        ),
    ];

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn identity_personal_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        title_selector(form, colors),
        labeled_input("First name", i.first_name.as_deref().unwrap_or(""), CipherFormMessage::IdentityFirstNameChanged, false, None, form.saving, colors,
        ),
        labeled_input("Middle name", i.middle_name.as_deref().unwrap_or(""), CipherFormMessage::IdentityMiddleNameChanged, false, None, form.saving, colors,
        ),
        labeled_input("Last name", i.last_name.as_deref().unwrap_or(""), CipherFormMessage::IdentityLastNameChanged, false, None, form.saving, colors,
        ),
        labeled_input("Username", i.username.as_deref().unwrap_or(""), CipherFormMessage::IdentityUsernameChanged, false, None, form.saving, colors,
        ),
        labeled_input("Company", i.company.as_deref().unwrap_or(""), CipherFormMessage::IdentityCompanyChanged, false, None, form.saving, colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn identity_identification_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        labeled_input("Social Security number", i.ssn.as_deref().unwrap_or(""), CipherFormMessage::IdentitySsnChanged, !form.identity_ssn_visible, Some((form.identity_ssn_visible, CipherFormMessage::ToggleIdentitySsnVisibility)), form.saving,
            colors,
        ),
        labeled_input("Passport number", i.passport_number.as_deref().unwrap_or(""), CipherFormMessage::IdentityPassportChanged, !form.identity_passport_visible, Some((
                form.identity_passport_visible, CipherFormMessage::ToggleIdentityPassportVisibility,  )),
            form.saving,
            colors,
        ),
        labeled_input("License number", i.license_number.as_deref().unwrap_or(""), CipherFormMessage::IdentityLicenseChanged, false, None, form.saving, colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn identity_contact_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        labeled_input("Email", i.email.as_deref().unwrap_or(""), CipherFormMessage::IdentityEmailChanged, false, None, form.saving, colors,
        ),
        labeled_input("Phone", i.phone.as_deref().unwrap_or(""), CipherFormMessage::IdentityPhoneChanged, false, None, form.saving, colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn identity_address_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let i = form.modified.identity.as_ref().expect("ensure_sub_structs");
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        labeled_input("Address line 1", i.address1.as_deref().unwrap_or(""), CipherFormMessage::IdentityAddress1Changed, false, None, form.saving, colors,
        ),
        labeled_input("Address line 2", i.address2.as_deref().unwrap_or(""), CipherFormMessage::IdentityAddress2Changed, false, None, form.saving, colors,
        ),
        labeled_input("Address line 3", i.address3.as_deref().unwrap_or(""), CipherFormMessage::IdentityAddress3Changed, false, None, form.saving, colors,
        ),
        labeled_input("City / town", i.city.as_deref().unwrap_or(""), CipherFormMessage::IdentityCityChanged, false, None, form.saving, colors,
        ),
        labeled_input("State / province", i.state.as_deref().unwrap_or(""), CipherFormMessage::IdentityStateChanged, false, None, form.saving, colors,
        ),
        labeled_input("Zip / postal code", i.postal_code.as_deref().unwrap_or(""), CipherFormMessage::IdentityPostalCodeChanged, false, None, form.saving, colors,
        ),
        labeled_input("Country", i.country.as_deref().unwrap_or(""), CipherFormMessage::IdentityCountryChanged, false, None, form.saving, colors,
        ),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn ssh_key_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    // Read-only: key editing + regeneration is out of scope for this pass.
    let default_key = SshKeyView {
        private_key: String::new(),
        public_key: String::new(),
        fingerprint: String::new(),
    };
    let k = form.modified.ssh_key.as_ref().unwrap_or(&default_key);
    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![
        field_readonly("Public key", k.public_key.clone(), colors),
        field_readonly("Private key", k.private_key.clone(), colors),
        field_readonly("Fingerprint", k.fingerprint.clone(), colors),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn additional_options_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let notes_value = form.modified.notes.as_deref().unwrap_or("");
    let notes = labeled_input("Notes", notes_value, CipherFormMessage::NotesChanged, false, None, form.saving, colors,
    );

    let reprompt_checkbox = checkbox(matches!(
        form.modified.reprompt,
        CipherRepromptType::Password
    ))
    .label("Password prompt")
    .on_toggle(|_| CipherFormMessage::RepromptToggled)
    .size(18)
    .spacing(8);

    let rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = vec![notes, reprompt_checkbox.into()];

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn custom_fields_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();

    let fields = form.modified.fields.as_deref().unwrap_or(&[]);

    if fields.is_empty() {
        rows.push(
            text("No custom fields yet")
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    } else {
        for (idx, f) in fields.iter().enumerate() {
            rows.push(custom_field_row(idx, f, form, colors));
        }
    }

    let add_btn = buttons::secondary(
        row![
            icons::PLUS.render(14.0, colors.accent),
            text("Add custom field").size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(CipherFormMessage::CustomFieldAdded)
    .padding([6, 12]);
    rows.push(add_btn.into());

    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn custom_field_row<'a>(
    idx: usize,
    f: &'a FieldView,
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let type_label = match f.r#type {
        FieldType::Text => "Text",
        FieldType::Hidden => "Hidden",
        FieldType::Boolean => "Boolean",
        FieldType::Linked => "Linked",
    };

    let type_trigger: Element<'a, CipherFormMessage, AppTheme> =
        bordered_dropdown_trigger(type_label, colors, move || {
            CipherFormMessage::CustomFieldTypeDropdownToggled(idx)
        });

    let type_options_panel: Element<'a, CipherFormMessage, AppTheme> = dropdown_panel(
        [FieldType::Text, FieldType::Hidden, FieldType::Boolean]
            .into_iter()
            .map(|ty| {
                let label = match ty {
                    FieldType::Text => "Text",
                    FieldType::Hidden => "Hidden",
                    FieldType::Boolean => "Boolean",
                    FieldType::Linked => "Linked",
                };
                (
                    label.to_string(),
                    CipherFormMessage::CustomFieldTypeSelected(idx, ty),
                )
            })
            .collect(),
        colors,
    );

    let is_open = self::dropdown_open(&form.custom_field_type_dropdowns, idx);
    let type_picker: Element<'a, CipherFormMessage, AppTheme> = DropDown::new(
        type_trigger,
        type_options_panel,
        is_open,
    )
    .alignment(drop_down::Alignment::BelowLeft)
    .on_dismiss(CipherFormMessage::CustomFieldTypeDropdownToggled(idx))
    .width(160.0)
    .offset(4.0)
    .into();

    let name_input = labeled_input(
        "Name",
        f.name.as_deref().unwrap_or(""),
        move |s| CipherFormMessage::CustomFieldNameChanged(idx, s),
        false,
        None,
        form.saving,
        colors,
    );

    let value_widget: Element<'a, CipherFormMessage, AppTheme> = match f.r#type {
        FieldType::Text => labeled_input(
            "Value",
            f.value.as_deref().unwrap_or(""),
            move |s| CipherFormMessage::CustomFieldValueChanged(idx, s),
            false,
            None,
            form.saving,
            colors,
        ),
        FieldType::Hidden => {
            let revealed = form.custom_field_reveals.get(&idx).copied().unwrap_or(false);
            labeled_input(
                "Value",
                f.value.as_deref().unwrap_or(""),
                move |s| CipherFormMessage::CustomFieldValueChanged(idx, s),
                !revealed,
                Some((revealed, CipherFormMessage::ToggleCustomFieldReveal(idx))),
                form.saving,
                colors,
            )
        }
        FieldType::Boolean => {
            let checked = matches!(f.value.as_deref(), Some("true"));
            checkbox(checked)
                .label("Enabled")
                .on_toggle(move |_| CipherFormMessage::CustomFieldBoolToggled(idx))
                .size(18)
                .spacing(8)
                .into()
        }
        FieldType::Linked => text("Linked fields not yet supported")
            .size(12)
            .color(colors.text_muted)
            .into(),
    };

    let remove_btn = buttons::ghost_icon(
        icons::BWI_TRASH.render(18.0, colors.titlebar_close_hover),
        colors.item_hover,
    )
    .on_press(CipherFormMessage::CustomFieldRemoved(idx))
    .padding([6, 6]);

    row![
        type_picker,
        container(name_input).width(Fill),
        container(value_widget).width(Fill),
        remove_btn,
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn dropdown_open(map: &HashMap<usize, bool>, idx: usize) -> bool {
    map.get(&idx).copied().unwrap_or(false)
}

// ── Selector helpers ───────────────────────────────────────────────────────

fn folder_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let selected_name = form
        .modified
        .folder_id
        .as_ref()
        .and_then(|id| form.folders.iter().find(|f| f.id == *id))
        .map(|f| f.name.as_str())
        .unwrap_or("No folder");

    let trigger =
        bordered_dropdown_trigger(selected_name, colors, || CipherFormMessage::FolderDropdownToggled);

    let mut options: Vec<(String, CipherFormMessage)> =
        vec![("No folder".to_string(), CipherFormMessage::FolderSelected(None))];
    for f in &form.folders {
        options.push((
            f.name.clone(),
            CipherFormMessage::FolderSelected(Some(f.id)),
        ));
    }

    labeled_dropdown(
        "Folder",
        trigger,
        dropdown_panel(options, colors),
        form.folder_dropdown_open,
        CipherFormMessage::FolderDropdownToggled,
        colors,
    )
}

fn org_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let selected_name = form
        .modified
        .organization_id
        .as_ref()
        .and_then(|id| form.organizations.iter().find(|o| o.id == *id))
        .map(|o| o.name.as_str())
        .unwrap_or("Personal (me)");

    let trigger =
        bordered_dropdown_trigger(selected_name, colors, || CipherFormMessage::OrgDropdownToggled);

    let mut options: Vec<(String, CipherFormMessage)> =
        vec![("Personal (me)".to_string(), CipherFormMessage::OrgSelected(None))];
    for o in &form.organizations {
        options.push((
            o.name.clone(),
            CipherFormMessage::OrgSelected(Some(o.id)),
        ));
    }

    labeled_dropdown(
        "Organization",
        trigger,
        dropdown_panel(options, colors),
        form.org_dropdown_open,
        CipherFormMessage::OrgDropdownToggled,
        colors,
    )
}

fn collections_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let org_id = form.modified.organization_id;
    let scoped: Vec<&CollectionOption> = form
        .collections
        .iter()
        .filter(|c| Some(c.organization_id) == org_id)
        .collect();

    let summary = if form.modified.collection_ids.is_empty() {
        "No collections".to_string()
    } else {
        let count = form.modified.collection_ids.len();
        format!("{count} selected")
    };

    let trigger = bordered_dropdown_trigger(&summary, colors, || {
        CipherFormMessage::CollectionsDropdownToggled
    });

    // Checkbox list panel
    let mut options: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();
    if scoped.is_empty() {
        options.push(
            container(text("No collections in this org").size(12).color(colors.text_muted))
                .padding([8, 12])
                .into(),
        );
    } else {
        for c in scoped {
            let selected = form.modified.collection_ids.contains(&c.id);
            let cid = c.id;
            options.push(
                buttons::ghost(
                    row![
                        checkbox(selected).size(16).spacing(0),
                        text(c.name.clone()).size(14).color(colors.text_primary),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                    false,
                    Color::TRANSPARENT,
                    colors.item_hover,
                    0.0,
                )
                .on_press(CipherFormMessage::CollectionToggled(cid))
                .padding([6, 12])
                .width(Fill)
                .into(),
            );
        }
    }

    let panel: Element<'a, CipherFormMessage, AppTheme> = container(column(options).spacing(0))
        .width(Fill)
        .padding([4, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(RADIUS_SM),
                )
        })
        .into();

    labeled_dropdown(
        "Collections",
        trigger,
        panel,
        form.collections_dropdown_open,
        CipherFormMessage::CollectionsDropdownToggled,
        colors,
    )
}

fn brand_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let current = form
        .modified
        .card
        .as_ref()
        .and_then(|c| c.brand.as_deref())
        .unwrap_or("-- Select --");

    let trigger = bordered_dropdown_trigger(current, colors, || {
        CipherFormMessage::CardBrandDropdownToggled
    });

    let brands = [
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
    ];
    let mut options: Vec<(String, CipherFormMessage)> = vec![(
        "-- Select --".to_string(),
        CipherFormMessage::CardBrandSelected(None),
    )];
    for b in brands {
        options.push((
            b.to_string(),
            CipherFormMessage::CardBrandSelected(Some(b.to_string())),
        ));
    }

    labeled_dropdown(
        "Brand",
        trigger,
        dropdown_panel(options, colors),
        form.card_brand_dropdown_open,
        CipherFormMessage::CardBrandDropdownToggled,
        colors,
    )
}

fn exp_month_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let current = form
        .modified
        .card
        .as_ref()
        .and_then(|c| c.exp_month.as_deref())
        .unwrap_or("-- Month --");

    let trigger = bordered_dropdown_trigger(current, colors, || {
        CipherFormMessage::CardExpMonthDropdownToggled
    });

    let mut options: Vec<(String, CipherFormMessage)> = vec![(
        "-- Month --".to_string(),
        CipherFormMessage::CardExpMonthSelected(None),
    )];
    for m in 1..=12 {
        let label = format!("{m:02}");
        options.push((
            label.clone(),
            CipherFormMessage::CardExpMonthSelected(Some(label)),
        ));
    }

    labeled_dropdown(
        "Expiration month",
        trigger,
        dropdown_panel(options, colors),
        form.card_exp_month_dropdown_open,
        CipherFormMessage::CardExpMonthDropdownToggled,
        colors,
    )
}

fn title_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let current = form
        .modified
        .identity
        .as_ref()
        .and_then(|i| i.title.as_deref())
        .unwrap_or("-- Title --");

    let trigger = bordered_dropdown_trigger(current, colors, || {
        CipherFormMessage::IdentityTitleDropdownToggled
    });

    let titles = ["Mr", "Mrs", "Ms", "Mx", "Dr"];
    let mut options: Vec<(String, CipherFormMessage)> = vec![(
        "-- Title --".to_string(),
        CipherFormMessage::IdentityTitleSelected(None),
    )];
    for t in titles {
        options.push((
            t.to_string(),
            CipherFormMessage::IdentityTitleSelected(Some(t.to_string())),
        ));
    }

    labeled_dropdown(
        "Title",
        trigger,
        dropdown_panel(options, colors),
        form.identity_title_dropdown_open,
        CipherFormMessage::IdentityTitleDropdownToggled,
        colors,
    )
}

// ── Generic dropdown primitives ────────────────────────────────────────────

fn bordered_dropdown_trigger<'a, M: Clone + 'a>(
    label: &str,
    colors: &'a AppColors,
    on_click: impl Fn() -> M + 'a,
) -> Element<'a, M, AppTheme> {
    let label_text = text(label.to_string()).size(14).color(colors.text_primary);
    let chevron = icons::BWI_ANGLE_DOWN.render(14.0, colors.text_secondary);
    let content = row![label_text, Space::new().width(Fill), chevron]
        .spacing(8)
        .align_y(Alignment::Center);

    buttons::ghost(
        container(content).padding([8, 12]).width(Fill),
        false,
        Color::TRANSPARENT,
        colors.item_hover,
        RADIUS_SM,
    )
    .on_press(on_click())
    .padding(0)
    .width(Fill)
    .style(|theme: &AppTheme, _status| {
        iced::widget::button::Style {
            background: Some(Background::Color(Color::TRANSPARENT)),
            text_color: theme.colors.text_primary,
            border: Border::default()
                .color(theme.colors.border)
                .width(1.0)
                .rounded(RADIUS_SM),
            shadow: iced::Shadow::default(),
            snap: false,
        }
    })
    .into()
}

fn dropdown_panel<'a, M: Clone + 'a>(
    options: Vec<(String, M)>,
    colors: &'a AppColors,
) -> Element<'a, M, AppTheme> {
    let items: Vec<Element<'a, M, AppTheme>> = options
        .into_iter()
        .map(|(label, msg)| {
            buttons::ghost(
                text(label).size(14).color(colors.text_primary),
                false,
                Color::TRANSPARENT,
                colors.item_hover,
                0.0,
            )
            .on_press(msg)
            .padding([6, 12])
            .width(Fill)
            .into()
        })
        .collect();

    container(column(items).spacing(0))
        .width(Fill)
        .padding([4, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(RADIUS_SM),
                )
        })
        .into()
}

fn labeled_dropdown<'a>(
    label: &'a str,
    trigger: Element<'a, CipherFormMessage, AppTheme>,
    panel: Element<'a, CipherFormMessage, AppTheme>,
    open: bool,
    dismiss_msg: CipherFormMessage,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    // Don't set a width on the DropDown — the overlay defaults to the
    // trigger's width (see `drop_down.rs` layout), which is what we want.
    // `Length::Fill` would stretch the overlay to the whole window.
    let dd: Element<'a, CipherFormMessage, AppTheme> = DropDown::new(trigger, panel, open)
        .alignment(drop_down::Alignment::BelowLeft)
        .on_dismiss(dismiss_msg)
        .offset(4.0)
        .into();

    // Plain "label above control" layout. The stack-based floating label
    // used by `floating_label_input` doubles layout cost per frame (see
    // CLAUDE.md → "Stack doesn't cull or clip"); at ~10 dropdowns per
    // form the extra layout passes add up noticeably during scroll.
    column![
        text(label).size(12).color(colors.text_muted),
        dd,
    ]
    .spacing(4)
    .into()
}


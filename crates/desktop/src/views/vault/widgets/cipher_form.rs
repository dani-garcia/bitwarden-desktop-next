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

use bitwarden_collections::collection::CollectionId;
use bitwarden_core::OrganizationId;
use bitwarden_vault::{
    CardView, CipherRepromptType, CipherType, CipherView, FieldType, FieldView, FolderId,
    FolderView, IdentityView, LoginUriView, LoginView, SecureNoteType, SecureNoteView, SshKeyView,
    UriMatchType,
};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length,
    widget::{Space, checkbox, column, combo_box, container, row, scrollable, text, text_editor},
};

use super::field_helpers::{card_with_margin, field_readonly, section_label, styled_card};
use crate::{
    components::{
        self, buttons, icons,
        inputs::{
            multi_select_field, reveal_text_field, search_select_field, select_field, text_field,
        },
    },
    fl,
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

/// Options shown in the searchable folder `combo_box`. `None` is a first-class
/// "No folder" entry so users can clear the selection by picking it — iced's
/// `combo_box` doesn't surface a separate clear signal.
#[derive(Debug, Clone)]
pub enum FolderChoice {
    None,
    Folder(FolderOption),
}

impl FolderChoice {
    fn id(&self) -> Option<FolderId> {
        match self {
            Self::None => None,
            Self::Folder(f) => Some(f.id),
        }
    }
}

impl std::fmt::Display for FolderChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str(&fl!("form-folder-none")),
            Self::Folder(fo) => f.write_str(&fo.name),
        }
    }
}

/// Options shown in the searchable organization `combo_box`. `None` is
/// the "Personal (me)" entry — personal ciphers have no organization.
#[derive(Debug, Clone)]
pub enum OrgChoice {
    None,
    Org(OrganizationOption),
}

impl OrgChoice {
    fn id(&self) -> Option<OrganizationId> {
        match self {
            Self::None => None,
            Self::Org(o) => Some(o.id),
        }
    }
}

impl std::fmt::Display for OrgChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::None => f.write_str(&fl!("form-organization-personal")),
            Self::Org(o) => f.write_str(&o.name),
        }
    }
}

/// Form backing the cipher edit view. Also sized to back the "New item"
/// flow (`original = None`), though wiring that is out of scope here.
pub struct CipherForm {
    pub original: Option<CipherView>,
    pub modified: CipherView,

    pub folders: Vec<FolderOption>,
    pub organizations: Vec<OrganizationOption>,
    pub collections: Vec<CollectionOption>,

    // `combo_box` holds its filter text / open state in a `RefCell<Inner<T>>`
    // that must persist across frames, so we keep the state on the struct
    // rather than recreating it in `view()`. Rebuilt by `set_folders` /
    // `set_organizations` when the SDK lists arrive.
    pub folder_combo_state: combo_box::State<FolderChoice>,
    pub org_combo_state: combo_box::State<OrgChoice>,

    // The collections multi-select still uses our custom DropDown widget
    // (iced's pick_list is single-select). Single-select dropdowns
    // (`select_field`) and reveal-toggle password fields (`reveal_text_field`)
    // manage their transient state inside iced's widget tree, so only the
    // multi-select needs an explicit flag here.
    pub collections_dropdown_open: bool,

    /// Multi-line editor buffer for the Notes field. `text_editor` requires
    /// its content/cursor state to live on the parent; mutations flow through
    /// `CipherFormMessage::NotesAction`.
    pub notes_content: text_editor::Content,

    /// Disables the Save button + form inputs while the save task is in flight.
    pub saving: bool,
}

impl CipherForm {
    /// Construct for editing an existing cipher. Clones the view so `original`
    /// stays pristine regardless of what form events do to `modified`.
    pub fn edit(cv: CipherView) -> Self {
        let notes_content = text_editor::Content::with_text(cv.notes.as_deref().unwrap_or(""));
        let mut form = Self {
            original: Some(cv.clone()),
            modified: cv,
            folders: Vec::new(),
            organizations: Vec::new(),
            collections: Vec::new(),
            folder_combo_state: combo_box::State::new(vec![FolderChoice::None]),
            org_combo_state: combo_box::State::new(vec![OrgChoice::None]),
            collections_dropdown_open: false,
            notes_content,
            saving: false,
        };
        form.ensure_sub_structs();
        form
    }

    /// Replace the folder list and rebuild the combo_box state. Uses
    /// `State::new` (rather than `State::with_selection`) so the internal
    /// `value` field starts empty — combo_box uses `value` as both the
    /// displayed text *and* the filter key, so a pre-filled value would
    /// reduce the next open to a single filtered row. The currently-selected
    /// folder is rendered via the `selected` argument passed into
    /// `search_select_field`; nothing in combo_box state needs to know it.
    pub fn set_folders(&mut self, folders: Vec<FolderOption>) {
        let mut choices = Vec::with_capacity(folders.len() + 1);
        choices.push(FolderChoice::None);
        choices.extend(folders.iter().cloned().map(FolderChoice::Folder));

        self.folder_combo_state = combo_box::State::new(choices);
        self.folders = folders;
    }

    /// Replace the organization list and rebuild the combo_box state. Same
    /// pattern as `set_folders` — `State::new` (empty value) so the next
    /// expand shows the full list unfiltered.
    pub fn set_organizations(&mut self, organizations: Vec<OrganizationOption>) {
        let mut choices = Vec::with_capacity(organizations.len() + 1);
        choices.push(OrgChoice::None);
        choices.extend(organizations.iter().cloned().map(OrgChoice::Org));

        self.org_combo_state = combo_box::State::new(choices);
        self.organizations = organizations;
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

    /// Router-triggered close helper; flips the collections multi-select
    /// closed. The single-select pick_lists manage their own overlay state.
    pub fn dismiss_dropdowns(&mut self) {
        self.collections_dropdown_open = false;
    }
}

// ── Messages ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum CipherFormMessage {
    // Item details (universal)
    NameChanged(String),
    NotesAction(text_editor::Action),
    FavoriteToggled,
    RepromptToggled,

    // Ownership
    FolderSelected(Option<FolderId>),
    FolderComboClosed,
    OrgSelected(Option<OrganizationId>),
    OrgComboClosed,
    CollectionsDropdownToggled,
    CollectionToggled(CollectionId),

    // Login
    UsernameChanged(String),
    PasswordChanged(String),
    TotpChanged(String),
    UriChanged(usize, String),
    UriAdded,
    UriRemoved(usize),

    // Card
    CardCardholderChanged(String),
    CardBrandSelected(Option<String>),
    CardNumberChanged(String),
    CardExpMonthSelected(Option<String>),
    CardExpYearChanged(String),
    CardCodeChanged(String),

    // Identity
    IdentityTitleSelected(Option<String>),
    IdentityFirstNameChanged(String),
    IdentityMiddleNameChanged(String),
    IdentityLastNameChanged(String),
    IdentityUsernameChanged(String),
    IdentityCompanyChanged(String),
    IdentitySsnChanged(String),
    IdentityPassportChanged(String),
    IdentityLicenseChanged(String),
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
    CustomFieldTypeSelected(usize, FieldType),
    CustomFieldNameChanged(usize, String),
    CustomFieldValueChanged(usize, String),
    CustomFieldBoolToggled(usize),

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
            NotesAction(action) => {
                // `text_editor` owns its buffer + cursor; we mirror the
                // plain-text value back onto the CipherView so saves / diff
                // logic keep working without knowing about the editor state.
                self.notes_content.perform(action);
                self.modified.notes = opt_string(self.notes_content.text());
            }
            FavoriteToggled => self.modified.favorite = !self.modified.favorite,
            RepromptToggled => {
                self.modified.reprompt = match self.modified.reprompt {
                    CipherRepromptType::None => CipherRepromptType::Password,
                    CipherRepromptType::Password => CipherRepromptType::None,
                };
            }

            // Ownership
            FolderSelected(id) => {
                self.modified.folder_id = id;
            }
            FolderComboClosed => {
                // combo_box has no public API to reset its internal `value`;
                // rebuilding the State via `set_folders` is the only way.
                let folders = std::mem::take(&mut self.folders);
                self.set_folders(folders);
            }
            OrgSelected(id) => {
                self.modified.organization_id = id;
                // Clearing the org clears any collections that were scoped to it.
                if id.is_none() {
                    self.modified.collection_ids.clear();
                } else {
                    self.modified.collection_ids.retain(|cid| {
                        self.collections
                            .iter()
                            .any(|c| &c.id == cid && Some(c.organization_id) == id)
                    });
                }
            }
            OrgComboClosed => {
                let orgs = std::mem::take(&mut self.organizations);
                self.set_organizations(orgs);
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
            CardBrandSelected(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.brand = s;
                }
            }
            CardNumberChanged(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.number = opt_string(s);
                }
            }
            CardExpMonthSelected(s) => {
                if let Some(c) = self.modified.card.as_mut() {
                    c.exp_month = s;
                }
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

            // Identity
            IdentityTitleSelected(s) => {
                if let Some(i) = self.modified.identity.as_mut() {
                    i.title = s;
                }
            }
            IdentityFirstNameChanged(s) => {
                identity_set(&mut self.modified, |i| &mut i.first_name, s)
            }
            IdentityMiddleNameChanged(s) => {
                identity_set(&mut self.modified, |i| &mut i.middle_name, s)
            }
            IdentityLastNameChanged(s) => identity_set(&mut self.modified, |i| &mut i.last_name, s),
            IdentityUsernameChanged(s) => identity_set(&mut self.modified, |i| &mut i.username, s),
            IdentityCompanyChanged(s) => identity_set(&mut self.modified, |i| &mut i.company, s),
            IdentitySsnChanged(s) => identity_set(&mut self.modified, |i| &mut i.ssn, s),
            IdentityPassportChanged(s) => {
                identity_set(&mut self.modified, |i| &mut i.passport_number, s)
            }
            IdentityLicenseChanged(s) => {
                identity_set(&mut self.modified, |i| &mut i.license_number, s)
            }
            IdentityEmailChanged(s) => identity_set(&mut self.modified, |i| &mut i.email, s),
            IdentityPhoneChanged(s) => identity_set(&mut self.modified, |i| &mut i.phone, s),
            IdentityAddress1Changed(s) => identity_set(&mut self.modified, |i| &mut i.address1, s),
            IdentityAddress2Changed(s) => identity_set(&mut self.modified, |i| &mut i.address2, s),
            IdentityAddress3Changed(s) => identity_set(&mut self.modified, |i| &mut i.address3, s),
            IdentityCityChanged(s) => identity_set(&mut self.modified, |i| &mut i.city, s),
            IdentityStateChanged(s) => identity_set(&mut self.modified, |i| &mut i.state, s),
            IdentityPostalCodeChanged(s) => {
                identity_set(&mut self.modified, |i| &mut i.postal_code, s)
            }
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
                }
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

fn identity_set(
    cv: &mut CipherView,
    pick: impl Fn(&mut IdentityView) -> &mut Option<String>,
    s: String,
) {
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
        section_label(fl!("form-section-item-details"), colors),
        item_details_card(form, colors),
    ];

    match form.modified.r#type {
        CipherType::Login => {
            sections.push(section_label(fl!("form-section-login-credentials"), colors));
            sections.push(login_card(form, colors));
            sections.push(section_label(fl!("form-section-autofill-options"), colors));
            sections.push(autofill_card(form, colors));
        }
        CipherType::Card => {
            sections.push(section_label(fl!("form-section-card-details"), colors));
            sections.push(card_details_card(form, colors));
        }
        CipherType::Identity => {
            sections.push(section_label(fl!("form-section-personal-details"), colors));
            sections.push(identity_personal_card(form, colors));
            sections.push(section_label(fl!("form-section-identification"), colors));
            sections.push(identity_identification_card(form, colors));
            sections.push(section_label(fl!("form-section-contact-info"), colors));
            sections.push(identity_contact_card(form, colors));
            sections.push(section_label(fl!("form-section-address"), colors));
            sections.push(identity_address_card(form, colors));
        }
        CipherType::SecureNote => { /* notes live in the shared "Additional options" card below */ }
        CipherType::SshKey => {
            sections.push(section_label(fl!("form-section-ssh-key"), colors));
            sections.push(ssh_key_card(form, colors));
        }
    }

    sections.push(section_label(fl!("form-section-additional-options"), colors));
    sections.push(additional_options_card(form, colors));

    sections.push(section_label(fl!("form-section-custom-fields"), colors));
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
            CipherType::Login => fl!("form-title-edit-login"),
            CipherType::Card => fl!("form-title-edit-card"),
            CipherType::Identity => fl!("form-title-edit-identity"),
            CipherType::SecureNote => fl!("form-title-edit-note"),
            CipherType::SshKey => fl!("form-title-edit-ssh-key"),
        }
    } else {
        fl!("form-title-new-item")
    };

    let title = text(title_text).size(18).color(colors.text_primary);

    let cancel_btn = buttons::ghost_icon(
        icons::BWI_CLOSE.render(32.0, colors.text_secondary),
        colors.item_hover,
    )
    .on_press(CipherFormMessage::Cancel)
    .padding([1, 1]);

    let header =
        container(row![title, Space::new().width(Fill), cancel_btn].align_y(Alignment::Center))
            .padding([8, 20]);

    column![header, components::separator_h()].spacing(0).into()
}

fn bottom_bar<'a>(
    form: &'a CipherForm,
    _colors: &AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let save_label = if form.saving { fl!("form-saving") } else { fl!("form-save") };
    let mut save_btn = buttons::primary(text(save_label).size(14)).padding([8, 20]);
    if !form.saving {
        save_btn = save_btn.on_press(CipherFormMessage::Save);
    }

    let cancel_btn = buttons::secondary(text(fl!("form-cancel")).size(14))
        .on_press(CipherFormMessage::Cancel)
        .padding([8, 20]);

    let bar = container(
        row![save_btn, cancel_btn]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([8, 20])
    .style(|theme: &AppTheme| container::Style::default().background(theme.colors.background));

    column![components::separator_h(), bar].spacing(0).into()
}

// ── Sections ───────────────────────────────────────────────────────────────

fn item_details_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let mut rows: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();

    rows.push(text_field(
        fl!("form-name"),
        &form.modified.name,
        CipherFormMessage::NameChanged,
        None,
        form.saving,
        colors,
    ));

    // Favorite + reprompt toggles in a row so the card stays compact.
    let favorite_checkbox = checkbox(form.modified.favorite)
        .label(fl!("form-favorite"))
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
        text_field(
            fl!("form-username"),
            login.username.as_deref().unwrap_or(""),
            CipherFormMessage::UsernameChanged,
            None,
            form.saving,
            colors,
        ),
        reveal_text_field(
            fl!("form-password"),
            login.password.as_deref().unwrap_or(""),
            CipherFormMessage::PasswordChanged,
            form.saving,
            colors,
        ),
        text_field(
            fl!("form-totp"),
            login.totp.as_deref().unwrap_or(""),
            CipherFormMessage::TotpChanged,
            None,
            form.saving,
            colors,
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
            text(fl!("form-uri-empty"))
                .size(14)
                .color(colors.text_muted)
                .into(),
        );
    } else {
        for (idx, uri) in uris.iter().enumerate() {
            let value = uri.uri.as_deref().unwrap_or("");
            let input = text_field(
                fl!("form-uri"),
                value,
                move |s| CipherFormMessage::UriChanged(idx, s),
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
                row![container(input).width(Fill), remove_btn,]
                    .spacing(6)
                    .align_y(Alignment::Center)
                    .into(),
            );
        }
    }

    let add_btn = buttons::secondary(
        row![
            icons::PLUS.render(14.0, colors.accent),
            text(fl!("form-add-website")).size(14),
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
        text_field(
            fl!("form-card-cardholder"),
            c.cardholder_name.as_deref().unwrap_or(""),
            CipherFormMessage::CardCardholderChanged,
            None,
            form.saving,
            colors,
        ),
        brand_selector(form, colors),
        text_field(
            fl!("form-card-number"),
            c.number.as_deref().unwrap_or(""),
            CipherFormMessage::CardNumberChanged,
            None,
            form.saving,
            colors,
        ),
        exp_month_selector(form, colors),
        text_field(
            fl!("form-card-exp-year"),
            c.exp_year.as_deref().unwrap_or(""),
            CipherFormMessage::CardExpYearChanged,
            None,
            form.saving,
            colors,
        ),
        reveal_text_field(
            fl!("form-card-code"),
            c.code.as_deref().unwrap_or(""),
            CipherFormMessage::CardCodeChanged,
            form.saving,
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

fn identity_identification_card<'a>(
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

fn identity_contact_card<'a>(
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

fn identity_address_card<'a>(
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
        field_readonly(fl!("form-ssh-public-key"), k.public_key.clone(), colors),
        field_readonly(fl!("form-ssh-private-key"), k.private_key.clone(), colors),
        field_readonly(fl!("form-ssh-fingerprint"), k.fingerprint.clone(), colors),
    ];
    card_with_margin(styled_card(column(rows).spacing(12).into()))
}

fn additional_options_card<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    // Multi-line notes field — `text_editor` grows with content between
    // `min_height` and `max_height`. Border/label come from `field_frame`
    // so the visual matches the single-line inputs; we strip the editor's
    // own border via `text_editor::Catalog`'s default style.
    let mut notes_editor = text_editor(&form.notes_content)
        .padding([10, 12])
        .height(Length::Shrink)
        .min_height(80.0)
        .max_height(600.0);
    if !form.saving {
        notes_editor = notes_editor.on_action(CipherFormMessage::NotesAction);
    }
    let notes =
        crate::components::inputs::field_frame(fl!("form-notes"), notes_editor.into(), colors);

    let reprompt_checkbox = checkbox(matches!(
        form.modified.reprompt,
        CipherRepromptType::Password
    ))
    .label(fl!("form-reprompt"))
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
            text(fl!("form-custom-field-empty"))
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
            text(fl!("form-add-custom-field")).size(14),
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
    let type_picker: Element<'a, CipherFormMessage, AppTheme> = container(select_field(
        fl!("form-custom-field-type"),
        Some(f.r#type),
        vec![FieldType::Text, FieldType::Hidden, FieldType::Boolean],
        |ty: &FieldType| match ty {
            FieldType::Text => fl!("form-custom-field-type-text"),
            FieldType::Hidden => fl!("form-custom-field-type-hidden"),
            FieldType::Boolean => fl!("form-custom-field-type-boolean"),
            FieldType::Linked => fl!("form-custom-field-type-linked"),
        },
        move |ty| CipherFormMessage::CustomFieldTypeSelected(idx, ty),
        colors,
    ))
    .width(140)
    .into();

    let name_input = text_field(
        fl!("form-custom-field-name"),
        f.name.as_deref().unwrap_or(""),
        move |s| CipherFormMessage::CustomFieldNameChanged(idx, s),
        None,
        form.saving,
        colors,
    );

    let value_widget: Element<'a, CipherFormMessage, AppTheme> = match f.r#type {
        FieldType::Text => text_field(
            fl!("form-custom-field-value"),
            f.value.as_deref().unwrap_or(""),
            move |s| CipherFormMessage::CustomFieldValueChanged(idx, s),
            None,
            form.saving,
            colors,
        ),
        FieldType::Hidden => reveal_text_field(
            fl!("form-custom-field-value"),
            f.value.as_deref().unwrap_or(""),
            move |s| CipherFormMessage::CustomFieldValueChanged(idx, s),
            form.saving,
            colors,
        ),
        FieldType::Boolean => {
            let checked = matches!(f.value.as_deref(), Some("true"));
            checkbox(checked)
                .label(fl!("form-custom-field-enabled"))
                .on_toggle(move |_| CipherFormMessage::CustomFieldBoolToggled(idx))
                .size(18)
                .spacing(8)
                .into()
        }
        FieldType::Linked => text(fl!("form-custom-field-linked-unsupported"))
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

// ── Selector helpers ───────────────────────────────────────────────────────

fn folder_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let selected = match form.modified.folder_id {
        None => FolderChoice::None,
        Some(id) => form
            .folders
            .iter()
            .find(|f| f.id == id)
            .cloned()
            .map(FolderChoice::Folder)
            .unwrap_or(FolderChoice::None),
    };
    let placeholder = selected.to_string();

    search_select_field(
        &form.folder_combo_state,
        fl!("form-folder"),
        placeholder,
        Some(selected),
        |choice: FolderChoice| CipherFormMessage::FolderSelected(choice.id()),
        CipherFormMessage::FolderComboClosed,
        colors,
    )
}

fn org_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
    let selected = match form.modified.organization_id {
        None => OrgChoice::None,
        Some(id) => form
            .organizations
            .iter()
            .find(|o| o.id == id)
            .cloned()
            .map(OrgChoice::Org)
            .unwrap_or(OrgChoice::None),
    };
    let placeholder = selected.to_string();

    search_select_field(
        &form.org_combo_state,
        fl!("form-organization"),
        placeholder,
        Some(selected),
        |choice: OrgChoice| CipherFormMessage::OrgSelected(choice.id()),
        CipherFormMessage::OrgComboClosed,
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
        fl!("form-collections-none")
    } else {
        let count = form.modified.collection_ids.len();
        fl!("form-collections-selected", count = count)
    };

    let trigger = bordered_dropdown_trigger(&summary, colors, || {
        CipherFormMessage::CollectionsDropdownToggled
    });

    // Checkbox list panel
    let mut options: Vec<Element<'a, CipherFormMessage, AppTheme>> = Vec::new();
    if scoped.is_empty() {
        options.push(
            container(
                text(fl!("form-collections-empty-in-org"))
                    .size(12)
                    .color(colors.text_muted),
            )
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

    multi_select_field(
        fl!("form-collections"),
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
        CipherFormMessage::CardBrandSelected,
        colors,
    )
}

fn exp_month_selector<'a>(
    form: &'a CipherForm,
    colors: &'a AppColors,
) -> Element<'a, CipherFormMessage, AppTheme> {
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
        CipherFormMessage::CardExpMonthSelected,
        colors,
    )
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
    .style(|theme: &AppTheme, _status| iced::widget::button::Style {
        background: Some(Background::Color(Color::TRANSPARENT)),
        text_color: theme.colors.text_primary,
        border: Border::default()
            .color(theme.colors.border)
            .width(1.0)
            .rounded(RADIUS_SM),
        shadow: iced::Shadow::default(),
        snap: false,
    })
    .into()
}


//! `CipherForm::update` — the big match that maps form messages onto
//! `CipherView` mutations.

use bitwarden_vault::{
    CipherRepromptType, CipherView, FieldType, FieldView, IdentityView, LoginUriView, UriMatchType,
};

use super::{
    message::{CipherEditMessage, FormEvent},
    state::CipherForm,
};

impl CipherForm {
    pub fn update(&mut self, msg: CipherEditMessage) -> FormEvent {
        use CipherEditMessage::*;
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
            PasskeyRemoved(idx) => {
                if let Some(l) = self.modified.login.as_mut()
                    && let Some(creds) = l.fido2_credentials.as_mut()
                    && idx < creds.len()
                {
                    creds.remove(idx);
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
            Save => return FormEvent::Save,
            Cancel => return FormEvent::Cancel,
        }
        FormEvent::None
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

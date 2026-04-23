//! `CipherForm` struct + choice enums + constructors/mutators.

use bitwarden_core::OrganizationId;
use bitwarden_vault::{
    CardView, CipherType, CipherView, FolderId, FolderView, IdentityView, LoginView,
    SecureNoteType, SecureNoteView,
};
use iced::widget::{combo_box, text_editor};

use crate::{
    fl,
    services::sdk::{Collection, Organization},
};

// ── Choice enums for searchable dropdowns ─────────────────────────────────

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
pub(super) enum FolderChoice {
    None,
    Folder(FolderOption),
}

impl FolderChoice {
    pub(super) fn id(&self) -> Option<FolderId> {
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
pub(super) enum OrgChoice {
    None,
    Org(OrganizationOption),
}

impl OrgChoice {
    pub(super) fn id(&self) -> Option<OrganizationId> {
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

// ── CipherForm struct ──────────────────────────────────────────────────────

/// Form backing the cipher edit view. Also sized to back the "New item"
/// flow (`original = None`), though wiring that is out of scope here.
pub struct CipherForm {
    pub original: Option<CipherView>,
    pub modified: CipherView,

    // All the folders/orgs/collections that the user has, used to filling the dropdowns
    pub folders: Vec<FolderOption>,
    pub organizations: Vec<OrganizationOption>,
    pub collections: Vec<CollectionOption>,

    // `combo_box` holds its filter text / open state in a `RefCell<Inner<T>>`
    // that must persist across frames, so we keep the state on the struct
    // rather than recreating it in `view()`. Rebuilt by `set_folders` /
    // `set_organizations` when the SDK lists arrive.
    pub(super) folder_combo_state: combo_box::State<FolderChoice>,
    pub(super) org_combo_state: combo_box::State<OrgChoice>,

    // The collections multi-select still uses our custom DropDown widget
    // (iced's pick_list is single-select). Single-select dropdowns
    // (`select_field`) and reveal-toggle password fields (`reveal_text_field`)
    // manage their transient state inside iced's widget tree, so only the
    // multi-select needs an explicit flag here.
    pub(super) collections_dropdown_open: bool,

    /// Multi-line editor buffer for the Notes field. `text_editor` requires
    /// its content/cursor state to live on the parent; mutations flow through
    /// `CipherFormMessage::NotesAction`.
    pub(super) notes_content: text_editor::Content,

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

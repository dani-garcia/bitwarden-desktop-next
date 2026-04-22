pub mod widgets;

use std::{collections::HashMap, sync::Arc};

use bitwarden_vault::{CipherId, CipherListView, CipherListViewType, CipherView};
use iced::{
    Alignment, Border, Element, Fill, Length, Padding, Task,
    widget::{Space, column, container, pane_grid, row, text},
};

use crate::{
    clipboard::Sensitivity,
    components::{
        account_switcher::{self, AccountEntry, AccountSwitcherMessage},
        bottom_sheet, buttons, icons, separator_v,
        toast::Toast,
        virtual_list,
    },
    fl,
    sdk::{ClientManager, Collection, Organization},
    state::{NavSection, SidebarFilter, SidebarMode, UserId},
    theme::{AppColors, AppTheme},
};

/// Below this window width (logical px) the detail pane renders as a
/// bottom sheet instead of a side-by-side `pane_grid` split.
pub const SHEET_BREAKPOINT_PX: f32 = 1000.0;

/// Visible strip at the top of the vault content above the bottom sheet
/// (≈2× `TITLE_BAR_HEIGHT`).
pub const SHEET_TOP_INSET_PX: f32 = 64.0;

/// Top-corner radius of the bottom sheet.
pub const SHEET_TOP_RADIUS_PX: f32 = 16.0;

use self::widgets::{
    cipher_form::{self, CipherForm, CipherFormMessage, FolderOption, FormAction},
    detail_pane::{self, DetailPaneMessage},
    item_list::{self, ItemListMessage},
    search_bar::{self, SearchMessage},
    sidebar::{self, SidebarMessage},
};

#[derive(Clone)]
pub enum VaultMessage {
    Sidebar(SidebarMessage),
    ItemList(ItemListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
    DetailPane(DetailPaneMessage),
    CipherForm(CipherFormMessage),
    CloseDetailPane,
    PaneResized(pane_grid::ResizeEvent),
    NewItem,
    /// User confirmed the delete in the modal — fire the SDK soft-delete.
    ConfirmDeleteSelected,
    /// User dismissed the delete modal (Cancel, backdrop click, etc.).
    CancelDeleteSelected,
    /// 1 Hz subscription tick — refreshes the TOTP code + countdown ring.
    TotpTick,
    /// Fires when the async `ClientManager::list_ciphers` task completes.
    ListLoaded(UserId, Result<Vec<Arc<CipherListView>>, String>),
    /// Fires when the async `ClientManager::full_cipher` task completes.
    DetailLoaded(UserId, CipherId, Result<Box<CipherView>, String>),
    /// Fires when `ClientManager::list_folders` completes for the form.
    /// Carries `FolderOption` (not `FolderView`) because `FolderView` isn't
    /// `Clone` and `VaultMessage` must be.
    FoldersLoaded(UserId, Result<Vec<FolderOption>, String>),
    /// Fires when orgs are loaded for the form.
    OrganizationsLoaded(UserId, Vec<Organization>),
    /// Fires when collections are loaded for the form.
    CollectionsLoaded(UserId, Vec<Collection>),
    /// Fires when `ClientManager::save_cipher` finishes.
    SaveCompleted(UserId, Result<Box<CipherView>, String>),
    /// Fires when `ClientManager::soft_delete_cipher` finishes.
    DeleteCompleted(UserId, CipherId, Result<(), String>),
}

// iced debug-formats every update Message and warns if it takes >1ms. The
// default `#[derive(Debug)]` on `ListLoaded` walks through ~20k entries of
// `Arc<CipherListView>` on the load-test account, which measured ~31 ms.
// The only variant that carries a genuinely heavy payload is `ListLoaded`
// (and the option loads for the form); everything else is short. Hand-roll
// Debug so bulk variants render as a terse summary and the cheap variants
// keep their natural derive-like output.
impl std::fmt::Debug for VaultMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Sidebar(m) => f.debug_tuple("Sidebar").field(m).finish(),
            Self::ItemList(m) => f.debug_tuple("ItemList").field(m).finish(),
            Self::Search(m) => f.debug_tuple("Search").field(m).finish(),
            Self::AccountSwitcher(m) => f.debug_tuple("AccountSwitcher").field(m).finish(),
            Self::DetailPane(m) => f.debug_tuple("DetailPane").field(m).finish(),
            Self::CipherForm(m) => f.debug_tuple("CipherForm").field(m).finish(),
            Self::CloseDetailPane => f.write_str("CloseDetailPane"),
            Self::PaneResized(e) => f.debug_tuple("PaneResized").field(e).finish(),
            Self::NewItem => f.write_str("NewItem"),
            Self::ConfirmDeleteSelected => f.write_str("ConfirmDeleteSelected"),
            Self::CancelDeleteSelected => f.write_str("CancelDeleteSelected"),
            Self::TotpTick => f.write_str("TotpTick"),
            Self::ListLoaded(uid, result) => {
                let mut t = f.debug_tuple("ListLoaded");
                t.field(uid);
                match result {
                    Ok(items) => t.field(&format_args!("Ok(<{} items>)", items.len())),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::DetailLoaded(uid, id, result) => {
                let mut t = f.debug_tuple("DetailLoaded");
                t.field(uid);
                t.field(id);
                match result {
                    Ok(_) => t.field(&"Ok(<CipherView>)"),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::FoldersLoaded(uid, result) => {
                let mut t = f.debug_tuple("FoldersLoaded");
                t.field(uid);
                match result {
                    Ok(items) => t.field(&format_args!("Ok(<{} folders>)", items.len())),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::OrganizationsLoaded(uid, orgs) => f
                .debug_tuple("OrganizationsLoaded")
                .field(uid)
                .field(&format_args!("<{} orgs>", orgs.len()))
                .finish(),
            Self::CollectionsLoaded(uid, cols) => f
                .debug_tuple("CollectionsLoaded")
                .field(uid)
                .field(&format_args!("<{} collections>", cols.len()))
                .finish(),
            Self::SaveCompleted(uid, result) => {
                let mut t = f.debug_tuple("SaveCompleted");
                t.field(uid);
                match result {
                    Ok(_) => t.field(&"Ok(<CipherView>)"),
                    Err(e) => t.field(&format_args!("Err({e})")),
                };
                t.finish()
            }
            Self::DeleteCompleted(uid, id, result) => f
                .debug_tuple("DeleteCompleted")
                .field(uid)
                .field(id)
                .field(result)
                .finish(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PaneKind {
    List,
    Detail,
}

// ── Sub-state ──────────────────────────────────────────────────────────────
//
// Three cohesive groups extracted from VaultView. Each group is a set of
// fields that are always read/written together. See docs/decisions.md →
// "View Architecture: Compositional MVU" for the rationale; this is the
// same grouping discipline applied one layer down.

/// Sidebar chrome + the filter it controls. Sidebar is the UI that owns
/// the active filter — the filter lives here, not on VaultView directly.
pub struct SidebarState {
    pub mode: SidebarMode,
    pub active_section: NavSection,
    pub active_filter: SidebarFilter,
    pub vault_tree_open: bool,
    pub send_tree_open: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            mode: SidebarMode::Expanded,
            active_section: NavSection::Vault,
            active_filter: SidebarFilter::AllItems,
            vault_tree_open: true,
            send_tree_open: false,
        }
    }
}

/// The currently-selected vault item. An item click sets index + id and
/// triggers an async decrypt that populates `detail`. When `form` is `Some`,
/// the right-hand pane renders the editable form instead of the read-only
/// detail view; `detail` stays populated throughout so cancel returns
/// instantly without a reload.
#[derive(Default)]
pub struct Selection {
    pub item: Option<usize>,
    pub id: Option<CipherId>,
    pub detail: Option<CipherView>,
    pub form: Option<CipherForm>,
    /// Armed-delete state for the detail pane's inline confirm row.
    /// Reset when selection changes (via `clear()`), when the user cancels,
    /// or when any non-delete detail-pane message arrives.
    pub confirm_delete: bool,
}

impl Selection {
    fn clear(&mut self) {
        *self = Self::default();
    }
}

/// Vault item storage. `all` is the full decrypted list from the SDK;
/// `cached` is `all` with the current filter + search applied. The two
/// are always kept in sync via `recompute()`.
#[derive(Default)]
pub struct ItemCache {
    pub all: Vec<Arc<CipherListView>>,
    pub cached: Vec<Arc<CipherListView>>,
}

// ── Events ─────────────────────────────────────────────────────────────────
//
// Declarative domain facts the vault bubbles up for cross-cutting effects.
// Routed by App. Anything that only affects the vault's own state stays
// inside `VaultView::update` and never becomes an event.

#[derive(Debug, Clone)]
pub enum VaultEvent {
    /// User picked a different account from the switcher.
    UserSelected { uid: UserId },
    /// User clicked Add Account — App switches to the login screen.
    AddAccountRequested,
    /// User clicked "Lock now" on the active account card.
    LockActiveRequested,
    /// User clicked "Log out" on the active account card.
    SignOutRequested,
    /// User clicked "Lock all accounts" in the account switcher options.
    LockAllRequested,
    /// User clicked "Settings" in the account switcher options.
    SettingsRequested,
    /// VaultView wants to show a cross-cutting toast notification.
    ToastRequested(Toast),
    /// User clicked a copy-to-clipboard button on the detail pane.
    /// Routed to `ClipboardManager::copy`; the handler also pushes a
    /// success toast with `toast_label` as the body.
    ClipboardCopyRequested {
        value: String,
        sensitivity: Sensitivity,
        toast_label: String,
    },
    /// User clicked the launch button on a login's URI.
    /// Routed to `clipboard::launch_url` which enforces scheme allowlist.
    LaunchUrlRequested { uri: String },
}

pub struct VaultView {
    pub search_query: String,
    pub account_switcher_open: bool,
    pub pane_state: pane_grid::State<PaneKind>,

    pub sidebar: SidebarState,
    pub selection: Selection,
    // Item storage keyed by user. Decrypted vault data is structurally
    // isolated per-user so it's impossible to accidentally show one user's
    // ciphers while another is active. `Arc` wrap because `CipherListView`
    // isn't `Clone` and both Message dispatch and filter recomputation
    // need cheap clones.
    pub items: HashMap<UserId, ItemCache>,
    /// Scroll state for the windowed item list. Updated by the
    /// `ItemListMessage::Scrolled` handler on every scroll event so
    /// `view()` can build only the visible row widgets.
    pub list_scroll: virtual_list::ScrollState,
}

impl VaultView {
    pub fn new() -> Self {
        // `list_pane` and the returned `detail_pane` are needed only as local
        // handles during construction (one to split from, one to resize).
        // `PaneKind::List` / `PaneKind::Detail` discriminants in `pane_state`
        // are what `view()` matches on at render time — we don't need the
        // `Pane` handles themselves after this.
        let (mut pane_state, list_pane) = pane_grid::State::new(PaneKind::List);
        let (_detail_pane, split_id) = pane_state
            .split(pane_grid::Axis::Vertical, list_pane, PaneKind::Detail)
            .expect("splitting a fresh single-pane state always succeeds");
        pane_state.resize(split_id, 0.4);

        Self {
            search_query: String::new(),
            account_switcher_open: false,
            pane_state,
            sidebar: SidebarState::default(),
            selection: Selection::default(),
            items: HashMap::new(),
            list_scroll: virtual_list::ScrollState::default(),
        }
    }

    /// Build the task that decrypts the user's vault list and lands as
    /// `VaultMessage::ListLoaded`. Called from App handlers (unlock, user
    /// switch, sync) — the factory lives here so all `Task::perform` calls
    /// that produce `VaultMessage`s stay within the owning view.
    pub fn load_list_task(uid: UserId, mgr: &Arc<ClientManager>) -> Task<VaultMessage> {
        let mgr = mgr.clone();
        Task::perform(
            async move {
                mgr.list_ciphers(&uid)
                    .await
                    .map(|items| items.into_iter().map(Arc::new).collect::<Vec<_>>())
            },
            move |result| VaultMessage::ListLoaded(uid, result),
        )
    }

    /// Compositional MVU update. Returns a task (for async work the view
    /// owns) and an optional event (cross-cutting fact for App to route).
    ///
    /// `client_manager` and `active_user` are injected at call-time so the
    /// view can construct `Task::perform` calls without owning shared state.
    pub fn update(
        &mut self,
        msg: VaultMessage,
        client_manager: &Arc<ClientManager>,
        active_user: Option<&UserId>,
    ) -> (Task<VaultMessage>, Option<VaultEvent>) {
        match msg {
            VaultMessage::Sidebar(sidebar_msg) => {
                match sidebar_msg {
                    SidebarMessage::FilterSelected(filter) => {
                        self.sidebar.active_filter = filter;
                        self.selection.clear();
                        if let Some(uid) = active_user {
                            self.recompute_filtered(uid);
                        }
                    }
                    SidebarMessage::ToggleSidebarMode => {
                        self.sidebar.mode = match self.sidebar.mode {
                            SidebarMode::Collapsed => SidebarMode::Expanded,
                            SidebarMode::Expanded => SidebarMode::Collapsed,
                        };
                    }
                    SidebarMessage::SectionSelected(section) => {
                        self.sidebar.active_section = section;
                    }
                    SidebarMessage::ToggleVaultTree => {
                        self.sidebar.vault_tree_open = !self.sidebar.vault_tree_open;
                    }
                    SidebarMessage::ToggleSendTree => {
                        self.sidebar.send_tree_open = !self.sidebar.send_tree_open;
                    }
                }
                (Task::none(), None)
            }
            VaultMessage::ItemList(item_msg) => match item_msg {
                ItemListMessage::ItemSelected(idx) => {
                    self.selection.item = Some(idx);
                    self.selection.id = active_user
                        .and_then(|uid| self.items.get(uid))
                        .and_then(|ic| ic.cached.get(idx))
                        .and_then(|i| i.id);
                    let Some(id) = self.selection.id else {
                        return (Task::none(), None);
                    };
                    let mgr = client_manager.clone();
                    let Some(uid) = active_user.copied() else {
                        return (Task::none(), None);
                    };
                    let task =
                        Task::perform(async move { mgr.full_cipher(&uid, id).await }, move |res| {
                            VaultMessage::DetailLoaded(uid, id, res.map(Box::new))
                        });
                    (task, None)
                }
                ItemListMessage::OpenExternal(_)
                | ItemListMessage::CopyUsername(_)
                | ItemListMessage::MoreOptions(_) => (Task::none(), None),
                ItemListMessage::Scrolled(viewport) => {
                    self.list_scroll.track(viewport);
                    (Task::none(), None)
                }
            },
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                if let Some(uid) = active_user {
                    self.recompute_filtered(uid);
                }
                (Task::none(), None)
            }
            VaultMessage::CloseDetailPane => {
                self.selection.clear();
                (Task::none(), None)
            }
            VaultMessage::PaneResized(event) => {
                self.pane_state.resize(event.split, event.ratio);
                (Task::none(), None)
            }
            VaultMessage::DetailPane(detail_msg) => match detail_msg {
                DetailPaneMessage::Edit => {
                    let Some(detail) = self.selection.detail.clone() else {
                        return (Task::none(), None);
                    };
                    let Some(uid) = active_user.copied() else {
                        return (Task::none(), None);
                    };
                    let form = CipherForm::edit(detail);
                    self.selection.form = Some(form);

                    // Kick off the three option-list loads in parallel.
                    // Folders go through the SDK repo; orgs + collections are
                    // pure reads off the already-loaded ClientManager, so we
                    // wrap them in `Task::done` to keep the message flow
                    // uniform with the async case.
                    let mgr = client_manager.clone();
                    let folders_task = Task::perform(
                        async move {
                            mgr.list_folders(&uid).await.map(|folders| {
                                folders.into_iter().map(FolderOption::from).collect()
                            })
                        },
                        move |res| VaultMessage::FoldersLoaded(uid, res),
                    );
                    let orgs = client_manager.list_organizations(&uid);
                    let cols = client_manager.list_collections(&uid);
                    let orgs_task = Task::done(VaultMessage::OrganizationsLoaded(uid, orgs));
                    let cols_task = Task::done(VaultMessage::CollectionsLoaded(uid, cols));
                    (Task::batch([folders_task, orgs_task, cols_task]), None)
                }
                DetailPaneMessage::CopyUsername => {
                    let value = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.login.as_ref())
                        .and_then(|l| l.username.as_deref())
                        .map(str::to_owned);
                    match value {
                        Some(v) => (
                            Task::none(),
                            Some(VaultEvent::ClipboardCopyRequested {
                                value: v,
                                sensitivity: Sensitivity::Normal,
                                toast_label: fl!("vault-toast-copied-username"),
                            }),
                        ),
                        None => (Task::none(), None),
                    }
                }
                DetailPaneMessage::CopyPassword => {
                    let value = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.login.as_ref())
                        .and_then(|l| l.password.as_deref())
                        .map(str::to_owned);
                    match value {
                        Some(v) => (
                            Task::none(),
                            Some(VaultEvent::ClipboardCopyRequested {
                                value: v,
                                sensitivity: Sensitivity::Sensitive,
                                toast_label: fl!("vault-toast-copied-password"),
                            }),
                        ),
                        None => (Task::none(), None),
                    }
                }
                DetailPaneMessage::CopyUrl(idx) => {
                    let value = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.login.as_ref())
                        .and_then(|l| detail_pane::login_uri_at(l, idx))
                        .map(str::to_owned);
                    match value {
                        Some(v) => (
                            Task::none(),
                            Some(VaultEvent::ClipboardCopyRequested {
                                value: v,
                                sensitivity: Sensitivity::Normal,
                                toast_label: fl!("vault-toast-copied-website"),
                            }),
                        ),
                        None => (Task::none(), None),
                    }
                }
                DetailPaneMessage::OpenUrl(idx) => {
                    let uri = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.login.as_ref())
                        .and_then(|l| detail_pane::login_uri_at(l, idx))
                        .map(str::to_owned);
                    match uri {
                        Some(uri) => (Task::none(), Some(VaultEvent::LaunchUrlRequested { uri })),
                        None => (Task::none(), None),
                    }
                }
                DetailPaneMessage::CopyTotp => {
                    // Recompute the code at the moment of copy so the
                    // clipboard holds a value that's still valid for ~30 s.
                    let secret = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.login.as_ref())
                        .and_then(|l| l.totp.as_deref())
                        .map(str::to_owned);
                    match secret
                        .and_then(|s| bitwarden_vault::generate_totp(s, None).ok().map(|r| r.code))
                    {
                        Some(code) => (
                            Task::none(),
                            Some(VaultEvent::ClipboardCopyRequested {
                                value: code,
                                sensitivity: Sensitivity::Sensitive,
                                toast_label: fl!("vault-toast-copied-totp"),
                            }),
                        ),
                        None => (Task::none(), None),
                    }
                }
                DetailPaneMessage::CopyCustomField(idx) => {
                    let field = self
                        .selection
                        .detail
                        .as_ref()
                        .and_then(|c| c.fields.as_deref())
                        .and_then(|fs| fs.get(idx));
                    let Some(field) = field else {
                        return (Task::none(), None);
                    };
                    let Some(value) = field.value.as_deref().map(str::to_owned) else {
                        return (Task::none(), None);
                    };
                    // Only hidden fields go through the sensitive path
                    // (clipboard gets cleared after a timeout); plain text
                    // and boolean fields are treated like a copied username.
                    let sensitivity = match field.r#type {
                        bitwarden_vault::FieldType::Hidden => Sensitivity::Sensitive,
                        _ => Sensitivity::Normal,
                    };
                    (
                        Task::none(),
                        Some(VaultEvent::ClipboardCopyRequested {
                            value,
                            sensitivity,
                            toast_label: fl!("vault-toast-copied-field"),
                        }),
                    )
                }
                DetailPaneMessage::Delete => {
                    // Open the confirm modal. Actual delete waits for the
                    // user to press Confirm (`ConfirmDeleteSelected`).
                    self.selection.confirm_delete = true;
                    (Task::none(), None)
                }
                // Close is intercepted at the caller's `.map()` and never
                // reaches this match — kept for exhaustiveness.
                DetailPaneMessage::Close => (Task::none(), None),
            },
            VaultMessage::CancelDeleteSelected => {
                self.selection.confirm_delete = false;
                (Task::none(), None)
            }
            VaultMessage::TotpTick => {
                // No state to mutate — the detail pane recomputes the TOTP
                // code and countdown from the current clock on each render,
                // so we just need the message to flow through `update()` to
                // trigger an iced redraw.
                (Task::none(), None)
            }
            VaultMessage::ConfirmDeleteSelected => {
                self.selection.confirm_delete = false;
                let Some(cipher_id) = self.selection.id else {
                    return (Task::none(), None);
                };
                let Some(uid) = active_user.copied() else {
                    return (Task::none(), None);
                };
                let mgr = client_manager.clone();
                let task = Task::perform(
                    async move { mgr.soft_delete_cipher(&uid, cipher_id).await },
                    move |res| VaultMessage::DeleteCompleted(uid, cipher_id, res),
                );
                (task, None)
            }
            VaultMessage::CipherForm(form_msg) => {
                let Some(form) = self.selection.form.as_mut() else {
                    return (Task::none(), None);
                };
                match form.update(form_msg) {
                    FormAction::None => (Task::none(), None),
                    FormAction::Cancel => {
                        self.selection.form = None;
                        (Task::none(), None)
                    }
                    FormAction::Save => {
                        let Some(uid) = active_user.copied() else {
                            return (Task::none(), None);
                        };
                        form.saving = true;
                        let mgr = client_manager.clone();
                        let cipher_view = form.modified.clone();
                        let task = Task::perform(
                            async move { mgr.save_cipher(&uid, cipher_view).await },
                            move |res| VaultMessage::SaveCompleted(uid, res.map(Box::new)),
                        );
                        (task, None)
                    }
                }
            }
            VaultMessage::FoldersLoaded(msg_uid, result) => {
                if active_user != Some(&msg_uid) {
                    return (Task::none(), None);
                }
                match result {
                    Ok(folders) => {
                        if let Some(form) = self.selection.form.as_mut() {
                            form.set_folders(folders);
                        }
                        (Task::none(), None)
                    }
                    Err(err) => {
                        tracing::error!(%err, "list_folders failed");
                        (Task::none(), None)
                    }
                }
            }
            VaultMessage::OrganizationsLoaded(msg_uid, orgs) => {
                if active_user != Some(&msg_uid) {
                    return (Task::none(), None);
                }
                if let Some(form) = self.selection.form.as_mut() {
                    form.set_organizations(orgs);
                }
                (Task::none(), None)
            }
            VaultMessage::CollectionsLoaded(msg_uid, cols) => {
                if active_user != Some(&msg_uid) {
                    return (Task::none(), None);
                }
                if let Some(form) = self.selection.form.as_mut() {
                    form.collections = cols;
                }
                (Task::none(), None)
            }
            VaultMessage::SaveCompleted(msg_uid, result) => {
                if active_user != Some(&msg_uid) {
                    return (Task::none(), None);
                }
                match result {
                    Ok(view) => {
                        self.selection.detail = Some(*view);
                        self.selection.form = None;
                        // Refresh the list so renamed items / ownership changes
                        // show up in the left pane's list without a manual reload.
                        let reload = Self::load_list_task(msg_uid, client_manager);
                        (
                            reload,
                            Some(VaultEvent::ToastRequested(Toast::success(
                                fl!("vault-toast-item-saved"),
                                None,
                            ))),
                        )
                    }
                    Err(err) => {
                        tracing::error!(%err, "save_cipher failed");
                        if let Some(form) = self.selection.form.as_mut() {
                            form.saving = false;
                        }
                        (
                            Task::none(),
                            Some(VaultEvent::ToastRequested(Toast::error(
                                fl!("vault-toast-save-failed-body"),
                                Some(&fl!("vault-toast-save-failed-title")),
                            ))),
                        )
                    }
                }
            }
            VaultMessage::DeleteCompleted(msg_uid, cipher_id, result) => {
                if active_user != Some(&msg_uid) {
                    return (Task::none(), None);
                }
                match result {
                    Ok(()) => {
                        self.selection.clear();
                        let reload = Self::load_list_task(msg_uid, client_manager);
                        (
                            reload,
                            Some(VaultEvent::ToastRequested(Toast::success(
                                fl!("vault-toast-item-deleted"),
                                None,
                            ))),
                        )
                    }
                    Err(err) => {
                        tracing::error!(cipher_id = %cipher_id, %err, "soft_delete_cipher failed");
                        (
                            Task::none(),
                            Some(VaultEvent::ToastRequested(Toast::error(
                                fl!("vault-toast-delete-failed-body"),
                                Some(&fl!("vault-toast-delete-failed-title")),
                            ))),
                        )
                    }
                }
            }
            VaultMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.account_switcher_open = !self.account_switcher_open;
                    (Task::none(), None)
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::UserSelected { uid }))
                }
                AccountSwitcherMessage::AddAccount => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::AddAccountRequested))
                }
                AccountSwitcherMessage::LockAll => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::LockAllRequested))
                }
                AccountSwitcherMessage::OpenSettings => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::SettingsRequested))
                }
                AccountSwitcherMessage::LockActive => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::LockActiveRequested))
                }
                AccountSwitcherMessage::LogOutActive => {
                    self.account_switcher_open = false;
                    (Task::none(), Some(VaultEvent::SignOutRequested))
                }
            },
            VaultMessage::NewItem => (Task::none(), None),
            VaultMessage::ListLoaded(msg_uid, result) => {
                match result {
                    Ok(items) => {
                        tracing::info!(
                            uid = %msg_uid,
                            count = items.len(),
                            "vault list loaded"
                        );
                        let cache = self.items.entry(msg_uid).or_default();
                        cache.all = items;
                        // Only recompute the filtered view if this is the
                        // active user — search_query / active_filter are
                        // view-global state that may not match a background
                        // user's context.
                        if active_user == Some(&msg_uid) {
                            self.recompute_filtered(&msg_uid);
                        }
                    }
                    Err(err) => {
                        tracing::error!(uid = %msg_uid, %err, "list_ciphers failed");
                    }
                }
                (Task::none(), None)
            }
            VaultMessage::DetailLoaded(msg_uid, id, result) => {
                // Stale-check: user switched while full_cipher was in flight.
                if active_user != Some(&msg_uid) {
                    tracing::debug!(
                        uid = %msg_uid,
                        cipher_id = %id,
                        "full_cipher result dropped: active user changed while in flight"
                    );
                    return (Task::none(), None);
                }
                match result {
                    Ok(view) => {
                        // Stale-check on the cipher id itself: if the user
                        // clicked a different item between the perform and the
                        // callback, drop the stale detail.
                        if self.selection.id == view.id {
                            self.selection.detail = Some(*view);
                        } else {
                            tracing::debug!(
                                cipher_id = %id,
                                "full_cipher result dropped: selection changed while in flight"
                            );
                        }
                        (Task::none(), None)
                    }
                    Err(err) => {
                        tracing::error!(cipher_id = %id, %err, "full_cipher failed");
                        (
                            Task::none(),
                            Some(VaultEvent::ToastRequested(Toast::error(
                                fl!("vault-toast-decrypt-failed-body"),
                                Some(&fl!("vault-toast-decrypt-failed-title")),
                            ))),
                        )
                    }
                }
            }
        }
    }

    /// LOAD-BEARING: called from the App router when a title-bar message
    /// arrives so the account-switcher dropdown closes. See the cross-view
    /// dismissal block in `App::update`.
    pub fn dismiss_dropdowns(&mut self) {
        self.account_switcher_open = false;
        if let Some(form) = self.selection.form.as_mut() {
            form.dismiss_dropdowns();
        }
    }

    /// `true` when the currently-selected cipher is a login with a
    /// non-empty `totp` field. Drives the 1 Hz subscription that refreshes
    /// the detail pane's TOTP code + countdown ring.
    pub fn has_totp_selected(&self) -> bool {
        self.selection
            .detail
            .as_ref()
            .and_then(|c| c.login.as_ref())
            .and_then(|l| l.totp.as_deref())
            .is_some_and(|s| !s.is_empty())
    }

    /// Recompute the filtered item list for a specific user. Uses the
    /// view-global `search_query` and `sidebar.active_filter` to derive
    /// `cached` from `all`. Also reconciles the selection index against
    /// the new filtered list.
    fn recompute_filtered(&mut self, uid: &UserId) {
        let query = self.search_query.to_lowercase();
        let filter = self.sidebar.active_filter;

        if let Some(cache) = self.items.get_mut(uid) {
            cache.cached = Self::filter_items(&cache.all, filter, &query);
        }

        if let Some(id) = self.selection.id {
            self.selection.item = self
                .items
                .get(uid)
                .and_then(|c| c.cached.iter().position(|i| i.id == Some(id)));
        } else {
            self.selection.item = None;
        }
        // Filter / search / reload shrinks or reshuffles the dataset —
        // reset the scroll offset so we don't end up past the new end.
        // The viewport_height stays cached from the last real scroll event.
        self.list_scroll.offset_y = 0.0;
    }

    /// Reset transient view state when switching users. Item caches are
    /// preserved in the map — keyed by user so they can't mix.
    pub fn reset(&mut self, uid: &UserId) {
        self.search_query.clear();
        self.selection.clear();
        self.sidebar.active_filter = SidebarFilter::AllItems;
        self.list_scroll = virtual_list::ScrollState::default();
        self.recompute_filtered(uid);
    }

    /// Remove a signed-out user's cached vault data.
    pub fn remove_user_items(&mut self, uid: &UserId) {
        self.items.remove(uid);
    }

    fn filter_items(
        all: &[Arc<CipherListView>],
        filter: SidebarFilter,
        query: &str,
    ) -> Vec<Arc<CipherListView>> {
        let items = all.iter().filter(|item| match filter {
            SidebarFilter::AllItems => true,
            SidebarFilter::Favorites => item.favorite,
            SidebarFilter::Category(cat) => cat == cipher_list_view_type_to_type(&item.r#type),
            SidebarFilter::Archive => item.archived_date.is_some(),
            SidebarFilter::Trash => item.deleted_date.is_some(),
        });

        if query.is_empty() {
            items.cloned().collect()
        } else {
            items
                .filter(|item| {
                    if item.name.to_lowercase().contains(query)
                        || item.subtitle.to_lowercase().contains(query)
                    {
                        return true;
                    }
                    if let CipherListViewType::Login(login) = &item.r#type
                        && let Some(uri) = login
                            .uris
                            .as_ref()
                            .and_then(|u| u.first())
                            .and_then(|u| u.uri.as_deref())
                        && uri.to_lowercase().contains(query)
                    {
                        return true;
                    }
                    false
                })
                .cloned()
                .collect()
        }
    }
}

/// Project a `CipherListViewType` down to its discriminant `CipherType`.
fn cipher_list_view_type_to_type(t: &CipherListViewType) -> bitwarden_vault::CipherType {
    use bitwarden_vault::CipherType;
    match t {
        CipherListViewType::Login(_) => CipherType::Login,
        CipherListViewType::SecureNote => CipherType::SecureNote,
        CipherListViewType::Card(_) => CipherType::Card,
        CipherListViewType::Identity => CipherType::Identity,
        CipherListViewType::SshKey => CipherType::SshKey,
    }
}

// ── View ───────────────────────────────────────────────────────────────────

impl VaultView {
    #[allow(clippy::too_many_arguments, reason = "view entry point threads session-wide context")]
    pub fn view<'a>(
        &'a self,
        active_user: &'a UserId,
        active_email: &'a str,
        accounts: &'a [AccountEntry],
        colors: &'a AppColors,
        window_width: f32,
        favicon: &'a crate::favicon::FaviconService,
        show_favicons: bool,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let cached_items: &[Arc<CipherListView>] = self
            .items
            .get(active_user)
            .map(|ic| ic.cached.as_slice())
            .unwrap_or(&[]);

        // --- Sidebar ---
        let sidebar = sidebar::view(&self.sidebar, colors).map(VaultMessage::Sidebar);

        // --- Content area ---
        // Wide window with detail open  → side-by-side `pane_grid` split.
        // No detail / narrow window     → list fills the area. (For the
        //   narrow case, `App::view_main` overlays the bottom sheet on top
        //   of the entire window — including sidebar and title bar — via
        //   `sheet_view()` below.)
        let show_pane_grid = self.selection.detail.is_some() && window_width >= SHEET_BREAKPOINT_PX;
        let content_area_inner: Element<'a, VaultMessage, AppTheme> = if show_pane_grid {
            pane_grid::PaneGrid::new(
                &self.pane_state,
                move |_pane, kind, _is_maximized| match kind {
                    PaneKind::List => pane_grid::Content::new(self.list_content(
                        cached_items,
                        active_email,
                        accounts,
                        colors,
                        favicon,
                        active_user,
                        show_favicons,
                    )),
                    PaneKind::Detail => {
                        let right_pane = self.detail_or_form_pane(colors, 0.0);
                        let with_separator = row![separator_v(), right_pane].height(Fill);
                        pane_grid::Content::new(with_separator)
                    }
                },
            )
            .on_resize(6, VaultMessage::PaneResized)
            .spacing(1)
            .min_size(250)
            .into()
        } else {
            self.list_content(
                cached_items,
                active_email,
                accounts,
                colors,
                favicon,
                active_user,
                show_favicons,
            )
        };

        let content_area = container(content_area_inner)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default()
                    .background(theme.colors.background)
                    .border(Border::default().rounded(iced::border::top_left(10)))
            });

        let main_row = container(row![sidebar, content_area].height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.header_bg)
            });

        container(main_row)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| {
                container::Style::default().background(theme.colors.background)
            })
            .into()
    }

    /// In narrow mode (`window_width < SHEET_BREAKPOINT_PX`) with a detail
    /// selected, returns the bottom-sheet element that the app composes on
    /// top of the entire window (including sidebar and title bar). Returns
    /// `None` otherwise.
    pub fn sheet_view<'a>(
        &'a self,
        colors: &'a AppColors,
        window_width: f32,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        if window_width >= SHEET_BREAKPOINT_PX {
            return None;
        }
        self.selection.detail.as_ref()?;
        let pane = self.detail_or_form_pane(colors, SHEET_TOP_RADIUS_PX);
        Some(bottom_sheet::view(
            pane,
            SHEET_TOP_INSET_PX,
            Some(VaultMessage::CloseDetailPane),
        ))
    }

    /// Returns the delete-confirmation modal when armed, `None` otherwise.
    /// Composed by the App on top of the vault view so the backdrop covers
    /// the sidebar and title bar.
    pub fn modal_view<'a>(
        &'a self,
        colors: &'a AppColors,
    ) -> Option<Element<'a, VaultMessage, AppTheme>> {
        if !self.selection.confirm_delete {
            return None;
        }
        let item_name = self
            .selection
            .detail
            .as_ref()
            .map(|c| c.name.as_str())
            .unwrap_or("");

        let title = text(fl!("vault-delete-modal-title"))
            .size(18)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);
        let body = text(fl!("vault-delete-modal-body", name = item_name))
            .size(14)
            .color(colors.text_primary);

        let cancel_btn = buttons::secondary(text(fl!("vault-delete-modal-cancel")).size(14))
            .on_press(VaultMessage::CancelDeleteSelected)
            .padding([8, 20]);
        let confirm_btn = buttons::primary(text(fl!("vault-delete-modal-confirm")).size(14))
            .on_press(VaultMessage::ConfirmDeleteSelected)
            .padding([8, 20]);

        let dialog_inner: Element<'_, VaultMessage, AppTheme> = column![
            title,
            body,
            row![Space::new().width(Fill), cancel_btn, confirm_btn]
                .spacing(8)
                .align_y(Alignment::Center),
        ]
        .spacing(12)
        .padding(Padding::from([16, 20]))
        .width(Length::Fixed(380.0))
        .into();

        let dialog = container(dialog_inner).style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.card_bg)
                .border(Border::default().rounded(crate::theme::RADIUS_LG))
        });

        Some(crate::components::modal::view(
            dialog.into(),
            VaultMessage::CancelDeleteSelected,
        ))
    }

    /// Builds the right-side pane content: either the editable `cipher_form`
    /// when `selection.form.is_some()`, or the read-only `detail_pane`.
    fn detail_or_form_pane<'a>(
        &'a self,
        colors: &'a AppColors,
        top_radius: f32,
    ) -> Element<'a, VaultMessage, AppTheme> {
        if let Some(form) = self.selection.form.as_ref() {
            cipher_form::view(form, colors, top_radius).map(VaultMessage::CipherForm)
        } else {
            let item = self
                .selection
                .detail
                .as_ref()
                .expect("detail_or_form_pane called without a selection");
            detail_pane::view(item, colors, top_radius).map(|msg| match msg {
                DetailPaneMessage::Close => VaultMessage::CloseDetailPane,
                other => VaultMessage::DetailPane(other),
            })
        }
    }

    /// Builds the list pane content (header + search + item list).
    #[allow(clippy::too_many_arguments, reason = "threading favicon state + ids through")]
    fn list_content<'a>(
        &'a self,
        cached_items: &'a [Arc<CipherListView>],
        active_email: &'a str,
        accounts: &'a [AccountEntry],
        colors: &'a AppColors,
        favicon: &'a crate::favicon::FaviconService,
        active_user: &'a UserId,
        show_favicons: bool,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let title = text(fl!("vault-title"))
            .size(28)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);

        let new_button = buttons::primary(
            row![
                icons::PLUS.render(14.0, colors.card_bg),
                text(fl!("vault-new-button")).size(14),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(VaultMessage::NewItem)
        .padding(Padding {
            top: 8.0,
            right: 16.0,
            bottom: 8.0,
            left: 12.0,
        });

        let avatar_trigger = account_switcher::avatar_trigger(active_email, colors)
            .map(VaultMessage::AccountSwitcher);
        let dd_panel = account_switcher::dropdown(Some(active_email), accounts, colors)
            .map(VaultMessage::AccountSwitcher);
        let avatar: Element<'a, VaultMessage, AppTheme> =
            crate::components::drop_down::DropDown::new(
                avatar_trigger,
                dd_panel,
                self.account_switcher_open,
            )
            .on_dismiss(VaultMessage::AccountSwitcher(
                AccountSwitcherMessage::ToggleDropdown,
            ))
            .alignment(crate::components::drop_down::Alignment::BelowRight)
            .width(360.0)
            .offset(4.0)
            .into();

        let content_header = container(
            row![title, Space::new().width(Fill), new_button, avatar]
                .spacing(12)
                .align_y(Alignment::Center),
        )
        .padding([16, 24])
        .width(Fill);

        let search = search_bar::view(&self.search_query).map(VaultMessage::Search);
        let search_row = container(search)
            .padding(Padding {
                top: 0.0,
                right: 24.0,
                bottom: 8.0,
                left: 24.0,
            })
            .width(Fill);

        let item_list = item_list::view(
            cached_items,
            self.selection.item,
            self.list_scroll,
            colors,
            favicon,
            active_user,
            show_favicons,
        )
        .map(VaultMessage::ItemList);

        column![content_header, search_row, item_list]
            .width(Fill)
            .height(Fill)
            .into()
    }
}


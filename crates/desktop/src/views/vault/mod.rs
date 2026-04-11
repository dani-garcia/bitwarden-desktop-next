pub mod widgets;

use std::sync::Arc;

use bitwarden_vault::{CipherId, CipherListView, CipherListViewType, CipherView};
use iced::{
    Alignment, Background, Border, Element, Fill, Padding, Task,
    widget::{Space, column, container, pane_grid, row, text},
};

use crate::{
    components::{
        account_switcher::{self, AccountEntry, AccountSwitcherMessage},
        buttons, icons,
        toast::Toast,
        virtual_list,
    },
    sdk::ClientManager,
    state::{NavSection, SidebarFilter, SidebarMode, UserId},
    theme::{AppColors, AppTheme},
};

use self::widgets::{
    detail_pane::{self, DetailPaneMessage},
    item_list::{self, ItemListMessage},
    search_bar::{self, SearchMessage},
    sidebar::{self, SidebarMessage},
};

#[derive(Debug, Clone)]
pub enum VaultMessage {
    Sidebar(SidebarMessage),
    ItemList(ItemListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
    DetailPane(#[expect(dead_code)] DetailPaneMessage),
    CloseDetailPane,
    PaneResized(pane_grid::ResizeEvent),
    NewItem,
    /// Fires when the async `ClientManager::list_ciphers` task completes.
    ListLoaded(UserId, Result<Vec<Arc<CipherListView>>, String>),
    /// Fires when the async `ClientManager::full_cipher` task completes.
    DetailLoaded(CipherId, Result<Box<CipherView>, String>),
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

/// The currently-selected vault item. All three fields are always set
/// and cleared together — an item click sets index + id + clears detail
/// (which then fills in from an async load), and closing the pane clears
/// all three.
#[derive(Default)]
pub struct Selection {
    pub item: Option<usize>,
    pub id: Option<CipherId>,
    pub detail: Option<CipherView>,
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
    /// User pressed Cmd/Ctrl-F or the menu Search shortcut. App runs
    /// `iced::widget::operation::focus(Id::new("vault-search"))` because
    /// focus operations don't produce a message and therefore can't live
    /// inside `VaultView::update` as a `Task<VaultMessage>`.
    SearchFocusRequested,
    /// VaultView wants to show a cross-cutting toast notification.
    #[expect(dead_code)] // No call sites yet; reserved for sync / copy / error flows.
    ToastRequested(Toast),
}

pub struct VaultView {
    pub search_query: String,
    pub dropdown_open: bool,
    pub pane_state: pane_grid::State<PaneKind>,

    pub sidebar: SidebarState,
    pub selection: Selection,
    // Item storage. `Arc` wrap because `CipherListView` isn't `Clone` and
    // both Message dispatch and filter recomputation need cheap clones.
    pub items: ItemCache,
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
            .unwrap();
        pane_state.resize(split_id, 0.4);

        Self {
            search_query: String::new(),
            dropdown_open: false,
            pane_state,
            sidebar: SidebarState::default(),
            selection: Selection::default(),
            items: ItemCache::default(),
            list_scroll: virtual_list::ScrollState::default(),
        }
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
                        self.recompute_filtered();
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
                    self.selection.id = self.items.cached.get(idx).and_then(|i| i.id);
                    self.selection.detail = None;
                    let Some(id) = self.selection.id else {
                        return (Task::none(), None);
                    };
                    let mgr = client_manager.clone();
                    let Some(uid) = active_user.cloned() else {
                        return (Task::none(), None);
                    };
                    let task = Task::perform(
                        async move { mgr.full_cipher(&uid, id).await },
                        move |res| VaultMessage::DetailLoaded(id, res.map(Box::new)),
                    );
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
                self.recompute_filtered();
                (Task::none(), Some(VaultEvent::SearchFocusRequested))
            }
            VaultMessage::CloseDetailPane => {
                self.selection.clear();
                (Task::none(), None)
            }
            VaultMessage::PaneResized(event) => {
                self.pane_state.resize(event.split, event.ratio);
                (Task::none(), None)
            }
            VaultMessage::DetailPane(_) => (Task::none(), None),
            VaultMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.dropdown_open = !self.dropdown_open;
                    (Task::none(), None)
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.dropdown_open = false;
                    (Task::none(), Some(VaultEvent::UserSelected { uid }))
                }
                AccountSwitcherMessage::AddAccount => {
                    self.dropdown_open = false;
                    (Task::none(), Some(VaultEvent::AddAccountRequested))
                }
            },
            VaultMessage::NewItem => (Task::none(), None),
            VaultMessage::ListLoaded(msg_uid, result) => {
                // Stale-check: user switched while list_ciphers was in flight.
                if active_user != Some(&msg_uid) {
                    tracing::debug!(
                        uid = %msg_uid,
                        "list_ciphers result dropped: active user changed while in flight"
                    );
                    return (Task::none(), None);
                }
                match result {
                    Ok(items) => {
                        tracing::info!(
                            uid = %msg_uid,
                            count = items.len(),
                            "vault list loaded"
                        );
                        self.set_items(items);
                    }
                    Err(err) => {
                        tracing::error!(uid = %msg_uid, %err, "list_ciphers failed");
                    }
                }
                (Task::none(), None)
            }
            VaultMessage::DetailLoaded(id, result) => match result {
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
                    (Task::none(), None)
                }
            },
        }
    }

    /// LOAD-BEARING: called from the App router when a title-bar message
    /// arrives so the account-switcher dropdown closes. See the cross-view
    /// dismissal block in `App::update`.
    pub fn dismiss_dropdowns(&mut self) {
        self.dropdown_open = false;
    }

    /// Replace the vault list with newly-decrypted items.
    fn set_items(&mut self, items: Vec<Arc<CipherListView>>) {
        self.items.all = items;
        self.recompute_filtered();
    }

    fn recompute_filtered(&mut self) {
        self.items.cached = self.filtered_items();
        if let Some(id) = self.selection.id {
            self.selection.item = self
                .items
                .cached
                .iter()
                .position(|i| i.id == Some(id));
        } else {
            self.selection.item = None;
        }
        // Filter / search / reload shrinks or reshuffles the dataset —
        // reset the scroll offset so we don't end up past the new end.
        // The viewport_height stays cached from the last real scroll event.
        self.list_scroll.offset_y = 0.0;
    }

    /// Reset transient state when switching users.
    pub fn reset(&mut self) {
        self.search_query.clear();
        self.selection.clear();
        self.sidebar.active_filter = SidebarFilter::AllItems;
        self.items = ItemCache::default();
        self.list_scroll = virtual_list::ScrollState::default();
    }

    fn filtered_items(&self) -> Vec<Arc<CipherListView>> {
        let items = self
            .items
            .all
            .iter()
            .filter(|item| match self.sidebar.active_filter {
                SidebarFilter::AllItems => true,
                SidebarFilter::Favorites => item.favorite,
                SidebarFilter::Category(cat) => cat == cipher_list_view_type_to_type(&item.r#type),
                SidebarFilter::Archive => item.archived_date.is_some(),
                SidebarFilter::Trash => item.deleted_date.is_some(),
            });

        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            items.cloned().collect()
        } else {
            items
                .filter(|item| {
                    if item.name.to_lowercase().contains(&query)
                        || item.subtitle.to_lowercase().contains(&query)
                    {
                        return true;
                    }
                    if let CipherListViewType::Login(login) = &item.r#type
                        && let Some(uri) = login
                            .uris
                            .as_ref()
                            .and_then(|u| u.first())
                            .and_then(|u| u.uri.as_deref())
                        && uri.to_lowercase().contains(&query)
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
    pub fn view<'a>(
        &'a self,
        active_email: &'a str,
        accounts: &'a [AccountEntry],
        colors: &'a AppColors,
    ) -> Element<'a, VaultMessage, AppTheme> {
        // --- Sidebar ---
        let sidebar = sidebar::view(&self.sidebar, colors).map(VaultMessage::Sidebar);

        // --- Content area: PaneGrid when detail open, plain list otherwise ---
        let content_area_inner: Element<'a, VaultMessage, AppTheme> = if self
            .selection
            .detail
            .is_some()
        {
            pane_grid::PaneGrid::new(&self.pane_state, move |_pane, kind, _is_maximized| match kind {
                PaneKind::List => {
                    pane_grid::Content::new(self.list_content(active_email, accounts, colors))
                }
                PaneKind::Detail => {
                    if let Some(item) = self.selection.detail.as_ref() {
                        let detail = detail_pane::view(item, colors).map(|msg| match msg {
                            DetailPaneMessage::Close => VaultMessage::CloseDetailPane,
                            other => VaultMessage::DetailPane(other),
                        });
                        pane_grid::Content::new(detail)
                    } else {
                        pane_grid::Content::new(Space::new())
                    }
                }
            })
            .on_resize(6, VaultMessage::PaneResized)
            .spacing(1)
            .min_size(250)
            .into()
        } else {
            self.list_content(active_email, accounts, colors)
        };

        let content_area = container(content_area_inner)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.background)),
                border: Border {
                    radius: iced::border::Radius {
                        top_left: 10.0,
                        top_right: 0.0,
                        bottom_right: 0.0,
                        bottom_left: 0.0,
                    },
                    ..Default::default()
                },
                ..Default::default()
            });

        let main_row = container(row![sidebar, content_area].height(Fill))
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.header_bg)),
                ..Default::default()
            });

        container(main_row)
            .width(Fill)
            .height(Fill)
            .style(|theme: &AppTheme| container::Style {
                background: Some(Background::Color(theme.colors.background)),
                ..Default::default()
            })
            .into()
    }

    /// Builds the list pane content (header + search + item list).
    fn list_content<'a>(
        &'a self,
        active_email: &'a str,
        accounts: &'a [AccountEntry],
        colors: &'a AppColors,
    ) -> Element<'a, VaultMessage, AppTheme> {
        let title = text("Vault")
            .size(28)
            .color(colors.text_primary)
            .font(crate::APP_FONT_BOLD);

        let new_button = buttons::primary(
            row![
                icons::PLUS.render(14.0, colors.card_bg),
                text("New").size(14),
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
        let dd_panel = account_switcher::dropdown(active_email, accounts, colors)
            .map(VaultMessage::AccountSwitcher);
        let avatar: Element<'a, VaultMessage, AppTheme> =
            crate::components::drop_down::DropDown::new(avatar_trigger, dd_panel, self.dropdown_open)
                .on_dismiss(VaultMessage::AccountSwitcher(
                    AccountSwitcherMessage::ToggleDropdown,
                ))
                .alignment(crate::components::drop_down::Alignment::BelowRight)
                .width(240.0)
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
            &self.items.cached,
            self.selection.item,
            self.list_scroll,
            colors,
        )
        .map(VaultMessage::ItemList);

        column![content_header, search_row, item_list]
            .width(Fill)
            .height(Fill)
            .into()
    }
}

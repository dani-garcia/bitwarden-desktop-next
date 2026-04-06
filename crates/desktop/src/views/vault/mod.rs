pub mod widgets;

use std::sync::Arc;

use bitwarden_vault::{CipherId, CipherListView, CipherListViewType, CipherView};
use iced::{
    Alignment, Background, Border, Element, Fill, Padding,
    widget::{Space, column, container, pane_grid, row, text},
};

use crate::{
    components::{
        account_switcher::{self, AccountEntry, AccountSwitcherMessage},
        buttons, icons,
    },
    state::{NavSection, SidebarFilter, SidebarMode},
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
}

#[derive(Debug, Clone, Copy)]
pub enum PaneKind {
    List,
    Detail,
}

#[derive(Debug, Clone)]
pub enum VaultAction {
    SwitchUser(String),
    FocusSearch,
    AddAccount,
    LoadDetail(CipherId),
    ClearDetail,
}

pub struct VaultView {
    pub search_query: String,
    pub active_filter: SidebarFilter,
    pub selected_item: Option<usize>,
    pub selected_id: Option<CipherId>,
    pub selected_detail: Option<CipherView>,
    pub dropdown_open: bool,
    pub sidebar_mode: SidebarMode,
    pub active_section: NavSection,
    pub vault_tree_open: bool,
    pub send_tree_open: bool,
    pub pane_state: pane_grid::State<PaneKind>,
    pub list_pane: pane_grid::Pane,
    pub detail_pane: pane_grid::Pane,
    // Cached data for view() — recomputed via set_items() / filtering on update.
    // Items are wrapped in `Arc` because `CipherListView` doesn't derive `Clone`,
    // and both Message dispatch and filter recomputation need cheap clones.
    pub cached_items: Vec<Arc<CipherListView>>,
    pub all_items: Vec<Arc<CipherListView>>,
}

impl VaultView {
    pub fn new() -> Self {
        let (mut pane_state, list_pane) = pane_grid::State::new(PaneKind::List);
        let (detail_pane, split_id) = pane_state
            .split(pane_grid::Axis::Vertical, list_pane, PaneKind::Detail)
            .unwrap();
        pane_state.resize(split_id, 0.4);

        Self {
            search_query: String::new(),
            active_filter: SidebarFilter::AllItems,
            selected_item: None,
            selected_id: None,
            selected_detail: None,
            dropdown_open: false,
            sidebar_mode: SidebarMode::Expanded,
            active_section: NavSection::Vault,
            vault_tree_open: true,
            send_tree_open: false,
            pane_state,
            list_pane,
            detail_pane,
            cached_items: Vec::new(),
            all_items: Vec::new(),
        }
    }

    pub fn update(&mut self, msg: VaultMessage) -> Vec<VaultAction> {
        let mut actions = Vec::new();
        match msg {
            VaultMessage::Sidebar(sidebar_msg) => match sidebar_msg {
                SidebarMessage::FilterSelected(filter) => {
                    self.active_filter = filter;
                    self.selected_item = None;
                }
                SidebarMessage::ToggleSidebarMode => {
                    self.sidebar_mode = match self.sidebar_mode {
                        SidebarMode::Collapsed => SidebarMode::Expanded,
                        SidebarMode::Expanded => SidebarMode::Collapsed,
                    };
                }
                SidebarMessage::SectionSelected(section) => {
                    self.active_section = section;
                }
                SidebarMessage::ToggleVaultTree => {
                    self.vault_tree_open = !self.vault_tree_open;
                }
                SidebarMessage::ToggleSendTree => {
                    self.send_tree_open = !self.send_tree_open;
                }
            },
            VaultMessage::ItemList(item_msg) => match item_msg {
                ItemListMessage::ItemSelected(idx) => {
                    self.selected_item = Some(idx);
                    self.selected_id = self.cached_items.get(idx).and_then(|i| i.id);
                    self.selected_detail = None;
                    if let Some(id) = self.selected_id {
                        actions.push(VaultAction::LoadDetail(id));
                    }
                }
                ItemListMessage::OpenExternal(_)
                | ItemListMessage::CopyUsername(_)
                | ItemListMessage::MoreOptions(_) => {}
            },
            VaultMessage::Search(SearchMessage::QueryChanged(query)) => {
                self.search_query = query;
                actions.push(VaultAction::FocusSearch);
            }
            VaultMessage::CloseDetailPane => {
                self.selected_item = None;
                self.selected_id = None;
                self.selected_detail = None;
                actions.push(VaultAction::ClearDetail);
            }
            VaultMessage::PaneResized(event) => {
                self.pane_state.resize(event.split, event.ratio);
            }
            VaultMessage::DetailPane(_) => {}
            VaultMessage::AccountSwitcher(asm) => match asm {
                AccountSwitcherMessage::ToggleDropdown => {
                    self.dropdown_open = !self.dropdown_open;
                }
                AccountSwitcherMessage::SwitchUser(uid) => {
                    self.dropdown_open = false;
                    actions.push(VaultAction::SwitchUser(uid));
                }
                AccountSwitcherMessage::AddAccount => {
                    self.dropdown_open = false;
                    actions.push(VaultAction::AddAccount);
                }
            },
            VaultMessage::NewItem => {}
        }
        actions
    }

    /// Replace the vault list with newly-decrypted items (called after `list_ciphers` finishes).
    pub fn set_items(&mut self, items: Vec<Arc<CipherListView>>) {
        self.all_items = items;
        self.recompute_filtered();
    }

    /// Apply a freshly-loaded `CipherView` if it still matches the user's current selection.
    /// Stale loads (the user clicked a different item before this one decrypted) are dropped.
    pub fn set_selected_detail(&mut self, view: CipherView) {
        if self.selected_id == view.id {
            self.selected_detail = Some(view);
        }
    }

    /// Refresh the filtered list (e.g. after a search query change).
    pub fn refresh_filter(&mut self) {
        self.recompute_filtered();
    }

    fn recompute_filtered(&mut self) {
        self.cached_items = self.filtered_items();
        if let Some(id) = self.selected_id {
            self.selected_item = self
                .cached_items
                .iter()
                .position(|i| i.id == Some(id));
        } else {
            self.selected_item = None;
        }
    }

    /// Reset transient state when switching users.
    pub fn reset(&mut self) {
        self.search_query.clear();
        self.selected_item = None;
        self.selected_id = None;
        self.selected_detail = None;
        self.active_filter = SidebarFilter::AllItems;
        self.all_items.clear();
        self.cached_items.clear();
    }

    fn filtered_items(&self) -> Vec<Arc<CipherListView>> {
        let items = self
            .all_items
            .iter()
            .filter(|item| match self.active_filter {
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

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    active_email: &'a str,
    _active_server: &'a str,
    items: &'a [Arc<CipherListView>],
    selected_item: Option<usize>,
    selected_detail: Option<&'a CipherView>,
    active_filter: SidebarFilter,
    search_query: &'a str,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
    sidebar_mode: SidebarMode,
    active_section: NavSection,
    vault_tree_open: bool,
    send_tree_open: bool,
    pane_state: &'a pane_grid::State<PaneKind>,
    _list_pane: pane_grid::Pane,
    _detail_pane: pane_grid::Pane,
    colors: &'a AppColors,
) -> Element<'a, VaultMessage, AppTheme> {
    // --- Sidebar ---
    let sidebar = sidebar::view(
        sidebar_mode,
        active_section,
        active_filter,
        vault_tree_open,
        send_tree_open,
        colors,
    )
    .map(VaultMessage::Sidebar);

    // --- Content area: PaneGrid when detail open, plain list otherwise ---
    let content_area_inner: Element<'a, VaultMessage, AppTheme> = if selected_detail.is_some() {
        pane_grid::PaneGrid::new(pane_state, |_pane, kind, _is_maximized| match kind {
            PaneKind::List => {
                let content = list_content(
                    active_email,
                    items,
                    selected_item,
                    search_query,
                    accounts,
                    dropdown_open,
                    colors,
                );
                pane_grid::Content::new(content)
            }
            PaneKind::Detail => {
                if let Some(item) = selected_detail {
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
        list_content(
            active_email,
            items,
            selected_item,
            search_query,
            accounts,
            dropdown_open,
            colors,
        )
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
    active_email: &'a str,
    items: &'a [Arc<CipherListView>],
    selected_item: Option<usize>,
    search_query: &'a str,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
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

    let avatar_trigger =
        account_switcher::avatar_trigger(active_email, colors).map(VaultMessage::AccountSwitcher);
    let dd_panel = account_switcher::dropdown(active_email, accounts, colors)
        .map(VaultMessage::AccountSwitcher);
    let avatar: Element<'a, VaultMessage, AppTheme> =
        crate::components::drop_down::DropDown::new(avatar_trigger, dd_panel, dropdown_open)
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

    let search = search_bar::view(search_query).map(VaultMessage::Search);
    let search_row = container(search)
        .padding(Padding {
            top: 0.0,
            right: 24.0,
            bottom: 8.0,
            left: 24.0,
        })
        .width(Fill);

    let item_list = item_list::view(items, selected_item, colors).map(VaultMessage::ItemList);

    column![content_header, search_row, item_list]
        .width(Fill)
        .height(Fill)
        .into()
}

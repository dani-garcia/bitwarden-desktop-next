pub mod widgets;

use iced::{
    Alignment, Background, Border, Element, Fill, Font, Padding,
    widget::{Space, column, container, pane_grid, row, text},
};

use crate::{
    components::{
        account_switcher::{self, AccountEntry, AccountSwitcherMessage},
        buttons, icons,
    },
    state::{CipherItem, NavSection, SidebarFilter, SidebarMode},
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
}

pub struct VaultView {
    pub search_query: String,
    pub active_filter: SidebarFilter,
    pub selected_item: Option<usize>,
    pub selected_id: Option<String>,
    pub dropdown_open: bool,
    pub sidebar_mode: SidebarMode,
    pub active_section: NavSection,
    pub vault_tree_open: bool,
    pub send_tree_open: bool,
    pub pane_state: pane_grid::State<PaneKind>,
    pub list_pane: pane_grid::Pane,
    pub detail_pane: pane_grid::Pane,
    // Cached data for view() — recomputed via refresh()
    pub cached_items: Vec<CipherItem>,
    pub all_items: Vec<CipherItem>,
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
                    self.selected_id = self.cached_items.get(idx).map(|i| i.id.clone());
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
            },
            VaultMessage::NewItem => {}
        }
        actions
    }

    /// Recompute cached items from the active session's vault data.
    pub fn refresh(&mut self, vault_items: &[CipherItem]) {
        self.all_items = vault_items.to_vec();
        self.cached_items = self.filtered_items();
        if let Some(ref id) = self.selected_id {
            self.selected_item = self.cached_items.iter().position(|i| i.id == *id);
        }
    }

    /// Reset transient state when switching users.
    pub fn reset(&mut self) {
        self.search_query.clear();
        self.selected_item = None;
        self.selected_id = None;
        self.active_filter = SidebarFilter::AllItems;
    }

    fn filtered_items(&self) -> Vec<CipherItem> {
        let items = self.all_items.iter().filter(|item| match self.active_filter {
            SidebarFilter::AllItems => true,
            SidebarFilter::Favorites => false,
            SidebarFilter::Category(cat) => item.category == cat,
            SidebarFilter::Archive => false,
            SidebarFilter::Trash => false,
        });

        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            items.cloned().collect()
        } else {
            items
                .filter(|item| {
                    item.name.to_lowercase().contains(&query)
                        || item
                            .username
                            .as_ref()
                            .is_some_and(|u| u.to_lowercase().contains(&query))
                        || item
                            .url
                            .as_ref()
                            .is_some_and(|u| u.to_lowercase().contains(&query))
                })
                .cloned()
                .collect()
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub fn view<'a>(
    active_email: &'a str,
    _active_server: &'a str,
    items: &'a [CipherItem],
    all_items: &'a [CipherItem],
    selected_item: Option<usize>,
    selected_id: Option<&'a str>,
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
    let selected_cipher = selected_id.and_then(|id| all_items.iter().find(|i| i.id == id));

    let content_area_inner: Element<'a, VaultMessage, AppTheme> = if selected_cipher.is_some() {
        pane_grid::PaneGrid::new(pane_state, |_pane, kind, _is_maximized| match kind {
            PaneKind::List => {
                let content = list_content(active_email, items, selected_item, search_query, accounts, dropdown_open, colors);
                pane_grid::Content::new(content)
            }
            PaneKind::Detail => {
                if let Some(item) = selected_cipher {
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
        list_content(active_email, items, selected_item, search_query, accounts, dropdown_open, colors)
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
    items: &'a [CipherItem],
    selected_item: Option<usize>,
    search_query: &'a str,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
    colors: &'a AppColors,
) -> Element<'a, VaultMessage, AppTheme> {
    let title = text("Vault")
        .size(28)
        .color(colors.text_primary)
        .font(Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        });

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

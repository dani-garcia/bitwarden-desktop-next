use iced::widget::{Space, button, column, container, pane_grid, row, stack, text};
use iced::{Alignment, Element, Fill, Font, Length, Padding};

use crate::app::PaneKind;
use crate::icons;
use crate::state::{CipherItem, NavSection, SidebarFilter, SidebarMode};
use crate::theme;
use crate::widgets::account_switcher::{self, AccountEntry, AccountSwitcherMessage};
use crate::widgets::detail_pane::{self, DetailPaneMessage};
use crate::widgets::item_list::{self, ItemListMessage};
use crate::widgets::search_bar::{self, SearchMessage};
use crate::widgets::sidebar::{self, SidebarMessage};

#[derive(Debug, Clone)]
#[allow(dead_code)] // DetailPane payload not yet consumed by handler
pub enum VaultMessage {
    Sidebar(SidebarMessage),
    ItemList(ItemListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
    DetailPane(DetailPaneMessage),
    CloseDetailPane,
    PaneResized(pane_grid::ResizeEvent),
    NewItem,
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
) -> Element<'a, VaultMessage> {
    // --- Sidebar ---
    let sidebar = sidebar::view(
        sidebar_mode,
        active_section,
        active_filter,
        vault_tree_open,
        send_tree_open,
    )
    .map(VaultMessage::Sidebar);

    // --- Content area: PaneGrid when detail open, plain list otherwise ---
    let selected_cipher = selected_id.and_then(|id| all_items.iter().find(|i| i.id == id));

    let content_area_inner: Element<'a, VaultMessage> = if selected_cipher.is_some() {
        pane_grid::PaneGrid::new(pane_state, |_pane, kind, _is_maximized| match kind {
            PaneKind::List => {
                let content = list_content(active_email, items, selected_item, search_query);
                pane_grid::Content::new(content)
            }
            PaneKind::Detail => {
                if let Some(item) = selected_cipher {
                    let detail = detail_pane::view(item).map(|msg| match msg {
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
        list_content(active_email, items, selected_item, search_query)
    };

    let content_area = container(content_area_inner)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BACKGROUND)),
            border: iced::Border {
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
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::HEADER_BG)),
            ..Default::default()
        });

    // --- Account switcher dropdown overlay ---
    let dropdown_layer: Element<'a, VaultMessage> = if dropdown_open {
        let dd =
            account_switcher::dropdown(active_email, accounts).map(VaultMessage::AccountSwitcher);
        column![
            Space::new().height(Length::Fixed(60.0)),
            container(dd).align_right(Fill).padding([0, 16]),
        ]
        .width(Fill)
        .into()
    } else {
        Space::new().width(0).height(0).into()
    };

    let layered = stack![main_row, dropdown_layer];

    container(layered)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BACKGROUND)),
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
) -> Element<'a, VaultMessage> {
    let title = text("Vault")
        .size(28)
        .color(theme::TEXT_PRIMARY)
        .font(Font {
            weight: iced::font::Weight::Bold,
            ..Font::DEFAULT
        });

    let new_button = button(
        row![
            icons::PLUS.render(14.0, theme::CARD_BG),
            text("New").size(14).color(theme::CARD_BG),
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
    })
    .style(|_theme, status| {
        let bg = match status {
            button::Status::Hovered => theme::ACCENT,
            _ => theme::BUTTON_PRIMARY,
        };
        button::Style {
            background: Some(iced::Background::Color(bg)),
            text_color: theme::TEXT_PRIMARY,
            border: iced::Border {
                radius: 20.0.into(),
                ..Default::default()
            },
            shadow: iced::Shadow::default(),
            snap: false,
        }
    });

    let avatar = account_switcher::avatar_trigger(active_email).map(VaultMessage::AccountSwitcher);

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

    let item_list = item_list::view(items, selected_item).map(VaultMessage::ItemList);

    column![content_header, search_row, item_list]
        .width(Fill)
        .height(Fill)
        .into()
}

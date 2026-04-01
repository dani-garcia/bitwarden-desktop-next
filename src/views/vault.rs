use iced::widget::{column, container, row, rule, stack, Space};
use iced::{Alignment, Element, Fill, Length, Padding};

use crate::state::{CipherItem, SidebarFilter};
use crate::theme;
use crate::widgets::account_switcher::{self, AccountEntry, AccountSwitcherMessage};
use crate::widgets::item_list::{self, ItemListMessage};
use crate::widgets::search_bar::{self, SearchMessage};
use crate::widgets::sidebar::{self, SidebarMessage};

#[derive(Debug, Clone)]
pub enum VaultMessage {
    Sidebar(SidebarMessage),
    ItemList(ItemListMessage),
    Search(SearchMessage),
    AccountSwitcher(AccountSwitcherMessage),
}

pub fn view<'a>(
    active_email: &'a str,
    active_server: &'a str,
    items: &'a [CipherItem],
    selected_item: Option<usize>,
    active_filter: SidebarFilter,
    search_query: &'a str,
    accounts: &'a [AccountEntry],
    dropdown_open: bool,
) -> Element<'a, VaultMessage> {
    let switcher_trigger =
        account_switcher::trigger(active_email, active_server, dropdown_open)
            .map(VaultMessage::AccountSwitcher);

    let search = search_bar::view(search_query).map(VaultMessage::Search);

    let top_bar = container(
        row![search, Space::new().width(Fill), switcher_trigger]
            .align_y(Alignment::Center)
            .padding(Padding::from([4.0, 8.0])),
    )
    .width(Fill)
    .style(|_theme| container::Style {
        background: Some(iced::Background::Color(theme::HEADER_BG)),
        ..Default::default()
    });

    let sidebar = sidebar::view(active_filter).map(VaultMessage::Sidebar);
    let item_list = item_list::view(items, selected_item).map(VaultMessage::ItemList);

    let divider = rule::vertical(1).style(|_theme| rule::Style {
        color: theme::BORDER,
        radius: 0.0.into(),
        fill_mode: rule::FillMode::Full,
        snap: false,
    });

    let main_content = row![sidebar, divider, item_list];
    let layout = column![top_bar, main_content].height(Fill);

    let dropdown_layer: Element<'a, VaultMessage> = if dropdown_open {
        let dd = account_switcher::dropdown(active_email, accounts)
            .map(VaultMessage::AccountSwitcher);
        column![
            Space::new().height(Length::Fixed(44.0)),
            container(dd).align_right(Fill).padding(Padding::from([0.0, 16.0])),
        ]
        .width(Fill)
        .into()
    } else {
        Space::new().width(0).height(0).into()
    };

    let layered = stack![layout, dropdown_layer];

    container(layered)
        .width(Fill)
        .height(Fill)
        .style(|_theme| container::Style {
            background: Some(iced::Background::Color(theme::BACKGROUND)),
            ..Default::default()
        })
        .into()
}

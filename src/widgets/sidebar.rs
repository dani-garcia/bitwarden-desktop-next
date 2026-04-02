use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding,
    widget::{Space, button, column, container, row, svg, text},
};

use crate::{
    icons,
    state::{CipherCategory, NavSection, SidebarFilter, SidebarMode},
    theme,
    widgets::common,
};

const RAIL_WIDTH: f32 = 50.0;
const PANEL_WIDTH: f32 = 232.0;
const RAIL_ICON_SIZE: f32 = 18.0;
const RAIL_BTN_SIZE: f32 = 36.0;
const ITEM_RADIUS: f32 = 6.0;
const SIDEBAR_H_PAD: f32 = 8.0;

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    FilterSelected(SidebarFilter),
    ToggleSidebarMode,
    SectionSelected(NavSection),
    ToggleVaultTree,
    ToggleSendTree,
}

pub fn view<'a>(
    mode: SidebarMode,
    active_section: NavSection,
    active_filter: SidebarFilter,
    vault_tree_open: bool,
    send_tree_open: bool,
) -> Element<'a, SidebarMessage> {
    if mode == SidebarMode::Expanded {
        expanded_panel(
            active_section,
            active_filter,
            vault_tree_open,
            send_tree_open,
        )
    } else {
        icon_rail(mode, active_section)
    }
}

// ---------------------------------------------------------------------------
// Icon Rail
// ---------------------------------------------------------------------------

fn icon_rail<'a>(_mode: SidebarMode, active_section: NavSection) -> Element<'a, SidebarMessage> {
    let shield = container(
        svg(svg::Handle::from_path("assets/bitwarden-shield.svg"))
            .width(28)
            .height(28),
    )
    .padding([12, 0])
    .width(RAIL_WIDTH)
    .align_x(Alignment::Center);

    let rail_btn = |icon: icons::BwiIcon, section: NavSection| -> Element<'a, SidebarMessage> {
        let is_active = active_section == section;
        button(
            container(icon.render(RAIL_ICON_SIZE, theme::TEXT_PRIMARY))
                .width(RAIL_BTN_SIZE)
                .height(RAIL_BTN_SIZE)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
        )
        .on_press(SidebarMessage::SectionSelected(section))
        .padding(0)
        .width(RAIL_WIDTH)
        .style(move |_theme, status| {
            common::hover_button_style(status, is_active, theme::SIDEBAR_SELECTED, ITEM_RADIUS)
        })
        .into()
    };

    let toggle_btn: Element<'a, SidebarMessage> = button(
        container(icons::BWI_ANGLE_RIGHT.render(32.0, theme::TEXT_SECONDARY))
            .align_x(Alignment::Center)
            .width(Fill),
    )
    .on_press(SidebarMessage::ToggleSidebarMode)
    .padding([6, 0])
    .width(Fill)
    .style(|_theme, status| {
        common::hover_button_style(status, false, Color::TRANSPARENT, ITEM_RADIUS)
    })
    .into();

    container(
        column![
            shield,
            rail_btn(icons::BWI_VAULT, NavSection::Vault),
            rail_btn(icons::BWI_SEND, NavSection::Send),
            rail_btn(icons::BWI_GENERATE, NavSection::Generator),
            rail_btn(icons::BWI_IMPORT, NavSection::Import),
            rail_btn(icons::BWI_DOWNLOAD, NavSection::Export),
            Space::new().height(Fill),
            container(toggle_btn).padding([0, 4]),
        ]
        .spacing(4)
        .align_x(Alignment::Center)
        .padding(Padding {
            top: 0.0,
            right: 4.0,
            bottom: 10.0,
            left: 4.0,
        }),
    )
    .width(Length::Fixed(RAIL_WIDTH))
    .height(Fill)
    .style(|_theme| container::Style {
        background: Some(Background::Color(theme::HEADER_BG)),
        border: Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

// ---------------------------------------------------------------------------
// Expanded Panel
// ---------------------------------------------------------------------------

fn expanded_panel<'a>(
    active_section: NavSection,
    active_filter: SidebarFilter,
    vault_tree_open: bool,
    send_tree_open: bool,
) -> Element<'a, SidebarMessage> {
    let mut items: Vec<Element<'a, SidebarMessage>> = Vec::new();

    // Logo header
    let logo = container(
        svg(svg::Handle::from_path("assets/password-manager-logo.svg"))
            .width(Fill)
            .height(Length::Shrink),
    )
    .padding(Padding {
        top: 10.0,
        right: 16.0,
        bottom: 14.0,
        left: 16.0,
    });
    items.push(logo.into());

    // Vault section (collapsible)
    items.push(section_header(
        "Vault",
        icons::BWI_VAULT,
        vault_tree_open,
        SidebarMessage::ToggleVaultTree,
        active_section == NavSection::Vault,
    ));

    if vault_tree_open {
        items.push(nav_button(
            "My Vault",
            icons::BWI_USER,
            SidebarFilter::AllItems,
            active_filter,
        ));
        items.push(nav_button(
            "Favorites",
            icons::BWI_STAR,
            SidebarFilter::Favorites,
            active_filter,
        ));
        items.push(nav_button(
            "Logins",
            icons::BWI_LOGIN,
            SidebarFilter::Category(CipherCategory::Login),
            active_filter,
        ));
        items.push(nav_button(
            "Cards",
            icons::BWI_CREDIT_CARD,
            SidebarFilter::Category(CipherCategory::Card),
            active_filter,
        ));
        items.push(nav_button(
            "Identities",
            icons::BWI_IDENTITY,
            SidebarFilter::Category(CipherCategory::Identity),
            active_filter,
        ));
        items.push(nav_button(
            "Notes",
            icons::BWI_NOTE,
            SidebarFilter::Category(CipherCategory::SecureNote),
            active_filter,
        ));
        items.push(nav_button(
            "SSH keys",
            icons::BWI_KEY,
            SidebarFilter::Category(CipherCategory::SshKey),
            active_filter,
        ));
        items.push(nav_button(
            "Archive",
            icons::BWI_ARCHIVE,
            SidebarFilter::Archive,
            active_filter,
        ));
        items.push(nav_button(
            "Trash",
            icons::BWI_TRASH,
            SidebarFilter::Trash,
            active_filter,
        ));
    }

    // Send section (collapsible)
    items.push(section_header(
        "Send",
        icons::BWI_SEND,
        send_tree_open,
        SidebarMessage::ToggleSendTree,
        active_section == NavSection::Send,
    ));

    // Standalone nav items
    items.push(standalone_item(
        "Generator",
        icons::BWI_GENERATE,
        NavSection::Generator,
        active_section,
    ));
    items.push(standalone_item(
        "Import",
        icons::BWI_IMPORT,
        NavSection::Import,
        active_section,
    ));
    items.push(standalone_item(
        "Export",
        icons::BWI_DOWNLOAD,
        NavSection::Export,
        active_section,
    ));

    // Collapse chevron at bottom
    let separator = container(common::separator_h()).padding([4.0, SIDEBAR_H_PAD]);

    let collapse_btn: Element<'a, SidebarMessage> = button(
        row![icons::BWI_ANGLE_LEFT.render(32.0, theme::TEXT_SECONDARY),].align_y(Alignment::Center),
    )
    .on_press(SidebarMessage::ToggleSidebarMode)
    .padding([8, 16])
    .width(Fill)
    .style(|_theme, status| {
        common::hover_button_style(status, false, Color::TRANSPARENT, ITEM_RADIUS)
    })
    .into();

    container(
        column![
            column(items).spacing(2).padding([0.0, SIDEBAR_H_PAD]),
            Space::new().height(Fill),
            separator,
            container(collapse_btn).padding([0.0, SIDEBAR_H_PAD]),
        ]
        .padding([10, 0]),
    )
    .width(Length::Fixed(PANEL_WIDTH))
    .height(Fill)
    .style(|_theme| container::Style {
        background: Some(Background::Color(theme::HEADER_BG)),
        border: Border {
            radius: 0.0.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn section_header<'a>(
    label: &'a str,
    icon: icons::BwiIcon,
    is_open: bool,
    toggle_msg: SidebarMessage,
    _is_active_section: bool,
) -> Element<'a, SidebarMessage> {
    let chevron = if is_open {
        icons::BWI_ANGLE_UP
    } else {
        icons::BWI_ANGLE_DOWN
    };
    button(
        row![
            icon.render(18.0, theme::TEXT_PRIMARY),
            text(label).size(16).color(theme::TEXT_PRIMARY),
            Space::new().width(Fill),
            chevron.render(21.0, theme::TEXT_SECONDARY),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(toggle_msg)
    .padding([8, 12])
    .width(Fill)
    .style(|_theme, status| {
        common::hover_button_style(status, false, Color::TRANSPARENT, ITEM_RADIUS)
    })
    .into()
}

fn nav_button<'a>(
    label: &'a str,
    icon: icons::BwiIcon,
    filter: SidebarFilter,
    active_filter: SidebarFilter,
) -> Element<'a, SidebarMessage> {
    let is_selected = active_filter == filter;

    button(
        row![
            icon.render(17.0, theme::TEXT_PRIMARY),
            text(label).size(15).color(theme::TEXT_PRIMARY)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .on_press(SidebarMessage::FilterSelected(filter))
    .padding(Padding {
        top: 6.0,
        right: 12.0,
        bottom: 6.0,
        left: 28.0,
    })
    .width(Fill)
    .style(move |_theme, status| {
        common::hover_button_style(status, is_selected, theme::SIDEBAR_SELECTED, ITEM_RADIUS)
    })
    .into()
}

fn standalone_item<'a>(
    label: &'a str,
    icon: icons::BwiIcon,
    section: NavSection,
    active_section: NavSection,
) -> Element<'a, SidebarMessage> {
    let is_active = active_section == section;
    let color = theme::TEXT_PRIMARY;

    button(
        row![icon.render(17.0, color), text(label).size(15).color(color)]
            .spacing(8)
            .align_y(Alignment::Center),
    )
    .on_press(SidebarMessage::SectionSelected(section))
    .padding([6, 12])
    .width(Fill)
    .style(move |_theme, status| {
        common::hover_button_style(status, is_active, theme::SIDEBAR_SELECTED, ITEM_RADIUS)
    })
    .into()
}

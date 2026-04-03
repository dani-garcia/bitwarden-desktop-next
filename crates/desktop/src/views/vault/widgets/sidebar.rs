use iced::{
    Alignment, Background, Border, Element, Fill, Length, Padding,
    widget::{Space, column, container, row, svg, text},
};

use crate::{
    components::{self, buttons, icons},
    state::{CipherCategory, NavSection, SidebarFilter, SidebarMode},
    theme::{AppColors, AppTheme},
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
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    if mode == SidebarMode::Expanded {
        expanded_panel(active_section, active_filter, vault_tree_open, send_tree_open, colors)
    } else {
        icon_rail(mode, active_section, colors)
    }
}

// ---------------------------------------------------------------------------
// Icon Rail
// ---------------------------------------------------------------------------

fn icon_rail<'a>(
    _mode: SidebarMode,
    active_section: NavSection,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let shield = container(
        svg(svg::Handle::from_path("assets/bitwarden-shield.svg"))
            .width(28)
            .height(28),
    )
    .padding([12, 0])
    .width(RAIL_WIDTH)
    .align_x(Alignment::Center);

    let nav_text = colors.nav_text;
    let sidebar_selected = colors.sidebar_selected;
    let nav_item_hover = colors.nav_item_hover;

    let rail_btn = move |icon: icons::BwiIcon, section: NavSection| -> Element<'a, SidebarMessage, AppTheme> {
        let is_active = active_section == section;
        buttons::ghost(
            container(icon.render(RAIL_ICON_SIZE, nav_text))
                .width(RAIL_BTN_SIZE)
                .height(RAIL_BTN_SIZE)
                .align_x(Alignment::Center)
                .align_y(Alignment::Center),
            is_active,
            sidebar_selected,
            nav_item_hover,
            ITEM_RADIUS,
        )
        .on_press(SidebarMessage::SectionSelected(section))
        .padding(0)
        .width(RAIL_WIDTH)
        .into()
    };

    let toggle_btn: Element<'a, SidebarMessage, AppTheme> = buttons::ghost_icon(
        container(icons::BWI_ANGLE_RIGHT.render(32.0, nav_text))
            .align_x(Alignment::Center)
            .width(Fill),
        nav_item_hover,
    )
    .on_press(SidebarMessage::ToggleSidebarMode)
    .padding([6, 0])
    .width(Fill)
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
    .style(|theme: &AppTheme| container::Style {
        background: Some(Background::Color(theme.colors.header_bg)),
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
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let mut items: Vec<Element<'a, SidebarMessage, AppTheme>> = Vec::new();

    // Logo header
    let logo = container(
        svg(svg::Handle::from_path("assets/password-manager-logo.svg"))
            .width(Fill)
            .height(Length::Shrink),
    )
    .padding(Padding {
        top: 4.0,
        right: 16.0,
        bottom: 6.0,
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
        colors,
    ));

    if vault_tree_open {
        items.push(nav_button("My Vault", icons::BWI_USER, SidebarFilter::AllItems, active_filter, colors));
        items.push(nav_button("Favorites", icons::BWI_STAR, SidebarFilter::Favorites, active_filter, colors));
        items.push(nav_button("Logins", icons::BWI_LOGIN, SidebarFilter::Category(CipherCategory::Login), active_filter, colors));
        items.push(nav_button("Cards", icons::BWI_CREDIT_CARD, SidebarFilter::Category(CipherCategory::Card), active_filter, colors));
        items.push(nav_button("Identities", icons::BWI_IDENTITY, SidebarFilter::Category(CipherCategory::Identity), active_filter, colors));
        items.push(nav_button("Notes", icons::BWI_NOTE, SidebarFilter::Category(CipherCategory::SecureNote), active_filter, colors));
        items.push(nav_button("SSH keys", icons::BWI_KEY, SidebarFilter::Category(CipherCategory::SshKey), active_filter, colors));
        items.push(nav_button("Archive", icons::BWI_ARCHIVE, SidebarFilter::Archive, active_filter, colors));
        items.push(nav_button("Trash", icons::BWI_TRASH, SidebarFilter::Trash, active_filter, colors));
    }

    // Send section (collapsible)
    items.push(section_header(
        "Send",
        icons::BWI_SEND,
        send_tree_open,
        SidebarMessage::ToggleSendTree,
        active_section == NavSection::Send,
        colors,
    ));

    // Standalone nav items
    items.push(standalone_item("Generator", icons::BWI_GENERATE, NavSection::Generator, active_section, colors));
    items.push(standalone_item("Import", icons::BWI_IMPORT, NavSection::Import, active_section, colors));
    items.push(standalone_item("Export", icons::BWI_DOWNLOAD, NavSection::Export, active_section, colors));

    // Collapse chevron at bottom
    let separator = container(components::separator_h()).padding([4.0, SIDEBAR_H_PAD]);

    let collapse_btn: Element<'a, SidebarMessage, AppTheme> = buttons::ghost_icon(
        row![icons::BWI_ANGLE_LEFT.render(32.0, colors.nav_text),].align_y(Alignment::Center),
        colors.nav_item_hover,
    )
    .on_press(SidebarMessage::ToggleSidebarMode)
    .padding([8, 16])
    .width(Fill)
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
    .style(|theme: &AppTheme| container::Style {
        background: Some(Background::Color(theme.colors.header_bg)),
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
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let chevron = if is_open {
        icons::BWI_ANGLE_UP
    } else {
        icons::BWI_ANGLE_DOWN
    };
    buttons::ghost_icon(
        row![
            icon.render(18.0, colors.nav_text),
            text(label).size(16).color(colors.nav_text),
            Space::new().width(Fill),
            chevron.render(21.0, colors.nav_text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        colors.nav_item_hover,
    )
    .on_press(toggle_msg)
    .padding([8, 12])
    .width(Fill)
    .into()
}

fn nav_button<'a>(
    label: &'a str,
    icon: icons::BwiIcon,
    filter: SidebarFilter,
    active_filter: SidebarFilter,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let is_selected = active_filter == filter;

    buttons::ghost(
        row![
            icon.render(17.0, colors.nav_text),
            text(label).size(16).color(colors.nav_text)
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        is_selected,
        colors.sidebar_selected,
        colors.nav_item_hover,
        ITEM_RADIUS,
    )
    .on_press(SidebarMessage::FilterSelected(filter))
    .padding(Padding {
        top: 6.0,
        right: 12.0,
        bottom: 6.0,
        left: 28.0,
    })
    .width(Fill)
    .into()
}

fn standalone_item<'a>(
    label: &'a str,
    icon: icons::BwiIcon,
    section: NavSection,
    active_section: NavSection,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let is_active = active_section == section;
    let color = colors.nav_text;

    buttons::ghost(
        row![icon.render(17.0, color), text(label).size(16).color(color)]
            .spacing(8)
            .align_y(Alignment::Center),
        is_active,
        colors.sidebar_selected,
        colors.nav_item_hover,
        ITEM_RADIUS,
    )
    .on_press(SidebarMessage::SectionSelected(section))
    .padding([6, 12])
    .width(Fill)
    .into()
}

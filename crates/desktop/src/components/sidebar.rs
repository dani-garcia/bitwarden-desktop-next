//! App-level sidebar chrome shared between the Vault and Send screens.
//!
//! State (mode, tree open flags, active filters) lives on `App` since the
//! sidebar persists across authenticated screens. Sidebar messages are
//! handled by `App`: section clicks may trigger a screen switch, and
//! per-screen filter changes are forwarded to the view that owns them.

use bitwarden_core::OrganizationId;
use bitwarden_vault::CipherType;
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Length, Padding,
    widget::{Space, column, container, row, svg, text},
};

use crate::{
    components::{self, buttons, icons},
    fl,
    services::sdk::Organization,
    theme::{AppColors, AppTheme},
};

const RAIL_WIDTH: f32 = 50.0;
const PANEL_WIDTH: f32 = 232.0;
const RAIL_ICON_SIZE: f32 = 18.0;
const RAIL_BTN_SIZE: f32 = 36.0;
const ITEM_RADIUS: f32 = 6.0;
const SIDEBAR_H_PAD: f32 = 8.0;

// ── Types ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SidebarMode {
    Collapsed,
    Expanded,
}

/// The "which screen / module am I in" nav facet. Generator / Import /
/// Export are placeholders without their own screens yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NavSection {
    Vault,
    Send,
    Generator,
    Import,
    Export,
}

/// Filter applied to the vault's cipher list.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VaultFilter {
    /// "Vault" parent — every cipher the user can see.
    AllItems,
    /// "My Vault" — personal ciphers only (not owned by an organization).
    Personal,
    Organization(OrganizationId),
    Favorites,
    Category(CipherType),
    Archive,
    Trash,
}

/// Filter applied to the send list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendFilter {
    /// "Send" parent — every send.
    AllItems,
    Text,
    File,
}

/// Sidebar chrome state that persists across screen switches.
pub struct SidebarState {
    pub mode: SidebarMode,
    pub active_section: NavSection,
    pub active_vault_filter: VaultFilter,
    pub active_send_filter: SendFilter,
    pub vault_tree_open: bool,
    pub send_tree_open: bool,
}

impl Default for SidebarState {
    fn default() -> Self {
        Self {
            mode: SidebarMode::Expanded,
            active_section: NavSection::Vault,
            active_vault_filter: VaultFilter::AllItems,
            active_send_filter: SendFilter::AllItems,
            vault_tree_open: true,
            send_tree_open: true,
        }
    }
}

#[derive(Debug, Clone)]
pub enum SidebarMessage {
    ToggleSidebarMode,
    SectionSelected(NavSection),
    ToggleVaultTree,
    ToggleSendTree,
    VaultFilterSelected(VaultFilter),
    SendFilterSelected(SendFilter),
}

// ── Entry point ────────────────────────────────────────────────────────────

pub fn view<'a>(
    state: &SidebarState,
    organizations: &[Organization],
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    if state.mode == SidebarMode::Expanded {
        expanded_panel(state, organizations, colors)
    } else {
        icon_rail(state.active_section, colors)
    }
}

// ── Icon rail (collapsed) ──────────────────────────────────────────────────

fn icon_rail<'a>(
    active_section: NavSection,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let shield = container(
        svg(svg::Handle::from_memory(crate::assets::BITWARDEN_SHIELD))
            .width(28)
            .height(28),
    )
    .padding([12, 0])
    .width(RAIL_WIDTH)
    .align_x(Alignment::Center);

    let nav_text = colors.nav_text;
    let sidebar_selected = colors.sidebar_selected;
    let nav_item_hover = colors.nav_item_hover;

    let rail_btn =
        move |icon: icons::BwiIcon, section: NavSection| -> Element<'a, SidebarMessage, AppTheme> {
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
    .style(|theme: &AppTheme| container::Style::default().background(theme.colors.header_bg))
    .into()
}

// ── Expanded panel ─────────────────────────────────────────────────────────

fn expanded_panel<'a>(
    state: &SidebarState,
    organizations: &[Organization],
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let active_section = state.active_section;
    let on_vault = active_section == NavSection::Vault;
    let on_send = active_section == NavSection::Send;

    let mut items: Vec<Element<'a, SidebarMessage, AppTheme>> = Vec::new();

    let logo = container(
        svg(svg::Handle::from_memory(
            crate::assets::PASSWORD_MANAGER_LOGO,
        ))
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

    items.push(parent_header_row(
        fl!("sidebar-section-vault"),
        icons::BWI_VAULT,
        state.vault_tree_open,
        on_vault && state.active_vault_filter == VaultFilter::AllItems,
        SidebarMessage::VaultFilterSelected(VaultFilter::AllItems),
        SidebarMessage::ToggleVaultTree,
        colors,
    ));

    if state.vault_tree_open {
        let active = if on_vault {
            Some(state.active_vault_filter)
        } else {
            None
        };
        items.push(vault_nav_button(
            fl!("sidebar-filter-my-vault"),
            icons::BWI_USER.render(17.0, colors.nav_text),
            VaultFilter::Personal,
            active,
            colors,
        ));
        for org in organizations {
            items.push(vault_nav_button(
                org.name.clone(),
                icons::BUILDING.render(17.0, colors.nav_text),
                VaultFilter::Organization(org.id),
                active,
                colors,
            ));
        }
        items.push(vault_nav_button(
            fl!("sidebar-filter-favorites"),
            icons::BWI_STAR.render(17.0, colors.nav_text),
            VaultFilter::Favorites,
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-logins"),
            icons::BWI_LOGIN.render(17.0, colors.nav_text),
            VaultFilter::Category(CipherType::Login),
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-cards"),
            icons::BWI_CREDIT_CARD.render(17.0, colors.nav_text),
            VaultFilter::Category(CipherType::Card),
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-identities"),
            icons::BWI_IDENTITY.render(17.0, colors.nav_text),
            VaultFilter::Category(CipherType::Identity),
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-notes"),
            icons::BWI_NOTE.render(17.0, colors.nav_text),
            VaultFilter::Category(CipherType::SecureNote),
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-ssh-keys"),
            icons::BWI_KEY.render(17.0, colors.nav_text),
            VaultFilter::Category(CipherType::SshKey),
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-archive"),
            icons::BWI_ARCHIVE.render(17.0, colors.nav_text),
            VaultFilter::Archive,
            active,
            colors,
        ));
        items.push(vault_nav_button(
            fl!("sidebar-filter-trash"),
            icons::BWI_TRASH.render(17.0, colors.nav_text),
            VaultFilter::Trash,
            active,
            colors,
        ));
    }

    items.push(parent_header_row(
        fl!("sidebar-section-send"),
        icons::BWI_SEND,
        state.send_tree_open,
        on_send && state.active_send_filter == SendFilter::AllItems,
        SidebarMessage::SendFilterSelected(SendFilter::AllItems),
        SidebarMessage::ToggleSendTree,
        colors,
    ));

    if state.send_tree_open {
        let active = if on_send {
            Some(state.active_send_filter)
        } else {
            None
        };
        items.push(send_nav_button(
            fl!("sidebar-filter-text-send"),
            icons::FILE_TEXT.render(17.0, colors.nav_text),
            SendFilter::Text,
            active,
            colors,
        ));
        items.push(send_nav_button(
            fl!("sidebar-filter-file-send"),
            icons::FILE_EARMARK.render(17.0, colors.nav_text),
            SendFilter::File,
            active,
            colors,
        ));
    }

    items.push(standalone_item(
        fl!("sidebar-item-generator"),
        icons::BWI_GENERATE.render(17.0, colors.nav_text),
        NavSection::Generator,
        active_section,
        colors,
    ));
    items.push(standalone_item(
        fl!("sidebar-item-import"),
        icons::BWI_IMPORT.render(17.0, colors.nav_text),
        NavSection::Import,
        active_section,
        colors,
    ));
    items.push(standalone_item(
        fl!("sidebar-item-export"),
        icons::BWI_DOWNLOAD.render(17.0, colors.nav_text),
        NavSection::Export,
        active_section,
        colors,
    ));

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
    .style(|theme: &AppTheme| container::Style::default().background(theme.colors.header_bg))
    .into()
}

// ── Shared row helpers ─────────────────────────────────────────────────────

/// Parent row for a collapsible section. The label side is a selectable
/// filter (`select_msg`); the chevron toggles the tree (`toggle_msg`). When
/// `is_selected`, an outer container paints the darkened background across
/// both halves.
fn parent_header_row<'a>(
    label: String,
    icon: icons::BwiIcon,
    is_open: bool,
    is_selected: bool,
    select_msg: SidebarMessage,
    toggle_msg: SidebarMessage,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    // When already selected, clicking the label is a no-op — suppress the
    // hover highlight. The chevron stays hoverable since toggling the tree
    // is still meaningful.
    let label_hover_bg = if is_selected {
        Color::TRANSPARENT
    } else {
        colors.nav_item_hover
    };
    let label_btn = buttons::ghost(
        row![
            icon.render(18.0, colors.nav_text),
            text(label).size(16).color(colors.nav_text),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        false,
        colors.sidebar_selected,
        label_hover_bg,
        ITEM_RADIUS,
    )
    .on_press(select_msg)
    .padding([8, 12])
    .width(Fill);

    let chevron = if is_open {
        icons::BWI_ANGLE_UP
    } else {
        icons::BWI_ANGLE_DOWN
    };
    let chevron_btn = buttons::ghost_icon(
        container(chevron.render(21.0, colors.nav_text))
            .padding([0, 8])
            .align_y(Alignment::Center),
        colors.nav_item_hover,
    )
    .on_press(toggle_msg)
    .padding([8, 4]);

    let inner = row![label_btn, chevron_btn]
        .spacing(0)
        .align_y(Alignment::Center);

    let selected_bg = colors.sidebar_selected;
    container(inner)
        .style(move |_theme: &AppTheme| {
            let bg = if is_selected {
                Background::Color(selected_bg)
            } else {
                Background::Color(Color::TRANSPARENT)
            };
            container::Style::default()
                .background(bg)
                .border(Border::default().rounded(ITEM_RADIUS))
        })
        .into()
}

fn vault_nav_button<'a>(
    label: impl Into<String>,
    icon: Element<'a, SidebarMessage, AppTheme>,
    filter: VaultFilter,
    active: Option<VaultFilter>,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    nav_row(
        label,
        icon,
        active == Some(filter),
        SidebarMessage::VaultFilterSelected(filter),
        colors,
    )
}

fn send_nav_button<'a>(
    label: impl Into<String>,
    icon: Element<'a, SidebarMessage, AppTheme>,
    filter: SendFilter,
    active: Option<SendFilter>,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    nav_row(
        label,
        icon,
        active == Some(filter),
        SidebarMessage::SendFilterSelected(filter),
        colors,
    )
}

fn nav_row<'a>(
    label: impl Into<String>,
    icon: Element<'a, SidebarMessage, AppTheme>,
    is_selected: bool,
    on_press: SidebarMessage,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    buttons::ghost(
        row![icon, text(label.into()).size(16).color(colors.nav_text)]
            .spacing(8)
            .align_y(Alignment::Center),
        is_selected,
        colors.sidebar_selected,
        colors.nav_item_hover,
        ITEM_RADIUS,
    )
    .on_press(on_press)
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
    label: impl Into<String>,
    icon: Element<'a, SidebarMessage, AppTheme>,
    section: NavSection,
    active_section: NavSection,
    colors: &AppColors,
) -> Element<'a, SidebarMessage, AppTheme> {
    let is_active = active_section == section;

    buttons::ghost(
        row![icon, text(label.into()).size(16).color(colors.nav_text)]
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

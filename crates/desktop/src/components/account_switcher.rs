use iced::{
    Alignment, Border, Color, Element, Fill,
    widget::{column, container, row, text},
};

use crate::{
    components::{self, buttons, icons},
    fl,
    state::UserId,
    theme::{AppColors, AppTheme, RADIUS_LG},
};

#[derive(Debug, Clone)]
pub enum AccountSwitcherMessage {
    ToggleDropdown,
    SwitchUser(UserId),
    AddAccount,
    LockAll,
    OpenSettings,
    LockActive,
    LogOutActive,
}

pub struct AccountEntry {
    pub user_id: UserId,
    pub email: String,
    #[expect(dead_code)] // Not displayed yet; reserved for future avatar / profile views.
    pub display_name: String,
    pub server_url: String,
    pub locked: bool,
}

const AVATAR_PALETTE: [Color; 5] = [
    Color::from_rgb8(0x00, 0x7c, 0x95), // teal
    Color::from_rgb8(0xc7, 0x18, 0x00), // coral
    Color::from_rgb8(0x17, 0x5d, 0xdc), // brand (blue)
    Color::from_rgb8(0x00, 0x82, 0x36), // green
    Color::from_rgb8(0x82, 0x00, 0xdb), // purple
];

/// Deterministic avatar color for a given id.
fn avatar_color_for(id: &str) -> Color {
    use std::hash::{Hash as _, Hasher as _};
    let mut hasher = std::hash::DefaultHasher::new();
    id.hash(&mut hasher);
    let hash = hasher.finish();

    AVATAR_PALETTE[hash as usize % AVATAR_PALETTE.len()]
}

/// Renders a round avatar circle trigger for the vault header.
pub fn avatar_trigger<'a>(
    active_email: &'a str,
    _colors: &AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    buttons::transparent(avatar(active_email, 36.0))
        .on_press(AccountSwitcherMessage::ToggleDropdown)
        .padding(0)
        .into()
}

/// Renders the floating dropdown panel.
///
/// Layout mirrors `designs/Desktop 2025/breakpoint-sm-authentication-1.png`
/// and `vault-1 or more accounts added-1..5.png`:
/// 1. Active account card — avatar, email + server, green check, "Lock now"
///    and "Log out" buttons. Only shown when the active account exists and
///    is unlocked.
/// 2. "Other Bitwarden accounts" list — one row per non-active account, with
///    a trailing lock / unlock icon. Row tap switches accounts.
/// 3. "Options" section — "Lock all accounts" (when any account is unlocked),
///    "Settings", "Add account".
///
/// Only `SwitchUser`, `AddAccount`, and `ToggleDropdown` are wired. The
/// Lock / Log out / Lock all / Settings buttons render visually but have no
/// `on_press` — they'll be wired in follow-up changes.
pub fn dropdown<'a>(
    active_email: Option<&'a str>,
    accounts: &'a [AccountEntry],
    colors: &'a AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    // Only show the active-account card when the account is unlocked — its
    // controls (Lock now / Log out) don't apply to a locked account. When
    // the active account is locked it falls back into the "Other" list so
    // the user can click it to bring up the unlock screen.
    let active_unlocked = active_email
        .and_then(|email| accounts.iter().find(|a| a.email == email))
        .filter(|a| !a.locked);
    let any_unlocked = accounts.iter().any(|a| !a.locked);

    let mut sections: Vec<Element<'a, AccountSwitcherMessage, AppTheme>> = Vec::new();

    if let Some(active) = active_unlocked {
        sections.push(active_account_card(active, colors));
        sections.push(section_divider());
    }

    let other_accounts: Vec<&AccountEntry> = accounts
        .iter()
        .filter(|a| Some(a.user_id) != active_unlocked.map(|x| x.user_id))
        .collect();

    if !other_accounts.is_empty() {
        sections.push(section_label(
            fl!("account-switcher-other-accounts"),
            colors,
        ));
        for entry in other_accounts {
            sections.push(other_account_row(entry, colors));
        }
        sections.push(section_divider());
    }

    sections.push(section_label(fl!("account-switcher-options"), colors));
    if any_unlocked {
        sections.push(options_row(
            icons::LOCK,
            fl!("account-switcher-lock-all"),
            Some(AccountSwitcherMessage::LockAll),
            colors,
        ));
    }
    sections.push(options_row(
        icons::GEAR,
        fl!("account-switcher-settings"),
        Some(AccountSwitcherMessage::OpenSettings),
        colors,
    ));
    sections.push(options_row(
        icons::PLUS,
        fl!("account-switcher-add"),
        Some(AccountSwitcherMessage::AddAccount),
        colors,
    ));

    container(column(sections).spacing(0))
        .width(360)
        .padding([8, 0])
        .style(|theme: &AppTheme| {
            container::Style::default()
                .background(theme.colors.background)
                .border(
                    Border::default()
                        .color(theme.colors.border)
                        .width(1.0)
                        .rounded(RADIUS_LG),
                )
        })
        .into()
}

// ── Internal building blocks ───────────────────────────────────────────────

/// Renders the initials avatar. Email keys both the initials (first two
/// chars) and the palette color. The Angular clients hash the UUID instead,
/// but cross-client color parity per user isn't required here — only palette
/// parity.
fn avatar<'a, M: 'a>(email: &str, size: f32) -> Element<'a, M, AppTheme> {
    let initials = email.chars().take(2).collect::<String>().to_uppercase();
    let bg = avatar_color_for(email);
    container(
        text(initials)
            .size(size * 0.4)
            .color(Color::WHITE)
            .font(crate::APP_FONT_BOLD),
    )
    .width(size)
    .height(size)
    .align_x(Alignment::Center)
    .align_y(Alignment::Center)
    .style(move |_theme: &AppTheme| {
        container::Style::default()
            .background(bg)
            .border(iced::border::rounded(size / 2.0))
    })
    .into()
}

fn active_account_card<'a>(
    entry: &'a AccountEntry,
    colors: &'a AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let identity = row![
        avatar(&entry.email, 40.0),
        column![
            text(&entry.email).size(14).color(colors.text_primary),
            text(&entry.server_url)
                .size(12)
                .color(colors.text_secondary),
        ]
        .spacing(2)
        .width(Fill),
        icons::CHECK_CIRCLE_FILL.render(20.0, colors.toast_success_bg),
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let lock_now = buttons::primary(
        row![
            icons::LOCK_FILL.render(12.0, colors.card_bg),
            text(fl!("account-switcher-lock-now")).size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(AccountSwitcherMessage::LockActive)
    .padding([8, 12])
    .width(Fill);

    let log_out = buttons::secondary(
        row![
            icons::BOX_ARROW_RIGHT.render(12.0, colors.accent),
            text(fl!("account-switcher-log-out")).size(14),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .on_press(AccountSwitcherMessage::LogOutActive)
    .padding([8, 12])
    .width(Fill);

    container(column![identity, row![lock_now, log_out].spacing(8)].spacing(12))
        .padding([8, 12])
        .width(Fill)
        .into()
}

fn other_account_row<'a>(
    entry: &'a AccountEntry,
    colors: &'a AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let uid = entry.user_id;
    let lock_icon = if entry.locked {
        icons::LOCK_FILL
    } else {
        icons::UNLOCK
    };

    buttons::ghost(
        row![
            avatar(&entry.email, 32.0),
            column![
                text(&entry.email).size(14).color(colors.text_primary),
                text(&entry.server_url)
                    .size(12)
                    .color(colors.text_secondary),
            ]
            .spacing(2)
            .width(Fill),
            lock_icon.render(16.0, colors.text_muted),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
        false,
        Color::TRANSPARENT,
        colors.item_hover,
        0.0,
    )
    .on_press(AccountSwitcherMessage::SwitchUser(uid))
    .padding([8, 12])
    .width(Fill)
    .into()
}

fn options_row<'a>(
    icon: icons::Icon,
    label: String,
    on_press: Option<AccountSwitcherMessage>,
    colors: &'a AppColors,
) -> Element<'a, AccountSwitcherMessage, AppTheme> {
    let button = buttons::ghost(
        row![
            icon.render(14.0, colors.accent),
            text(label).size(14).color(colors.accent),
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        false,
        Color::TRANSPARENT,
        colors.item_hover,
        0.0,
    )
    .padding([8, 12])
    .width(Fill);

    match on_press {
        Some(msg) => button.on_press(msg).into(),
        None => button.into(),
    }
}

fn section_label<'a, M: 'a>(label: String, colors: &AppColors) -> Element<'a, M, AppTheme> {
    container(text(label).size(12).color(colors.text_muted))
        .padding([8, 12])
        .into()
}

fn section_divider<'a, M: 'a>() -> Element<'a, M, AppTheme> {
    container(components::separator_h()).padding([4, 12]).into()
}

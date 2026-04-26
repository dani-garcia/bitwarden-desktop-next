use std::sync::Arc;

use bitwarden_vault::{CipherListView, CipherListViewType};
use iced::{
    Alignment, Background, Border, Color, Element, Fill, Shadow,
    widget::{
        Space, column, container, image, row, scrollable, text,
        text::{Ellipsis, Wrapping},
    },
};

use crate::{
    components::{self, buttons, icons, virtual_list},
    fl,
    services::favicon::{self, IconState},
    theme::{AppColors, AppTheme, RADIUS_MD, RADIUS_SM},
};

/// Total pixel height of a single vault row, including its trailing separator.
///
/// MUST match the actual rendered row height — `virtual_list` computes the
/// visible window in multiples of this constant, so drift would break the
/// scroll thumb ratio and cause rows to pop in or out at the wrong spot.
/// The row itself is `content_height + button padding (8+8) = 48`, plus a
/// 1px separator rendered as part of each row to keep the unit uniform.
const ROW_HEIGHT: f32 = 49.0;

#[derive(Debug, Clone)]
pub enum ItemListMessage {
    ItemSelected(usize),
    OpenExternal(#[expect(dead_code)] usize),
    CopyUsername(#[expect(dead_code)] usize),
    MoreOptions(#[expect(dead_code)] usize),
    /// Emitted on every scroll of the virtualized list. The vault handler
    /// forwards the viewport straight into `ScrollState::track`.
    Scrolled(scrollable::Viewport),
}

fn initial_color(name: &str) -> iced::Color {
    let hash: u32 = name
        .bytes()
        .fold(0u32, |acc, b| acc.wrapping_mul(31).wrapping_add(b as u32));
    let hue = (hash % 360) as f32;
    let s: f32 = 0.5;
    let l: f32 = 0.45;
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let x = c * (1.0 - ((hue / 60.0) % 2.0 - 1.0).abs());
    let m = l - c / 2.0;
    let (r, g, b) = match hue as u32 {
        0..=59 => (c, x, 0.0),
        60..=119 => (x, c, 0.0),
        120..=179 => (0.0, c, x),
        180..=239 => (0.0, x, c),
        240..=299 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    Color::from_rgb(r + m, g + m, b + m)
}

pub fn view<'a>(
    items: &'a [Arc<CipherListView>],
    selected_index: Option<usize>,
    scroll: virtual_list::ScrollState,
    ctx: &crate::app::RenderCtx<'a>,
) -> Element<'a, ItemListMessage, AppTheme> {
    let colors = ctx.colors;
    let table_header = container(
        row![
            text(fl!("vault-column-name"))
                .size(14)
                .color(colors.table_header)
                .font(crate::APP_FONT_BOLD),
            icons::ARROW_DOWN_UP.render(11.0, colors.table_header),
            Space::new().width(Fill),
            text(fl!("vault-column-options"))
                .size(14)
                .color(colors.table_header)
                .font(crate::APP_FONT_BOLD),
        ]
        .spacing(4)
        .align_y(Alignment::Center),
    )
    .padding([8, 24])
    .width(Fill);

    let header_divider = container(components::separator_h()).padding([0, 16]);

    // Virtualized body: only items inside the visible window (+ an auto
    // overscan buffer) become widgets. Top/bottom `Space` fillers preserve
    // the scroll thumb so it looks and feels like a real N-item list.
    let list = virtual_list::view(
        items,
        scroll,
        ROW_HEIGHT,
        |i, item| row_element(i, item, selected_index == Some(i), ctx),
        ItemListMessage::Scrolled,
    )
    .height(Fill)
    .style(|theme: &AppTheme, _status| scrollable::Style {
        container: container::Style::default(),
        vertical_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(theme.colors.item_hover),
                border: Border::default().rounded(4),
            },
        },
        horizontal_rail: scrollable::Rail {
            background: None,
            border: Border::default(),
            scroller: scrollable::Scroller {
                background: Background::Color(theme.colors.item_hover),
                border: Border::default().rounded(4),
            },
        },
        gap: None,
        auto_scroll: scrollable::AutoScroll {
            background: Background::Color(Color::TRANSPARENT),
            border: Border::default(),
            shadow: Shadow::default(),
            icon: Color::TRANSPARENT,
        },
    });

    column![table_header, header_divider, list]
        .width(Fill)
        .height(Fill)
        .into()
}

/// Build a single row Element at the given global index. `virtual_list`
/// wraps the returned Element in a `Length::Fixed(ROW_HEIGHT)` container
/// itself, so no height concern leaks into here.
fn row_element<'a>(
    i: usize,
    item: &'a CipherListView,
    is_selected: bool,
    ctx: &crate::app::RenderCtx<'a>,
) -> Element<'a, ItemListMessage, AppTheme> {
    let colors = ctx.colors;
    let favicon = ctx.favicon;
    let active_user = ctx.active_user.expect("item_list requires active_user");
    let show_favicons = ctx.show_favicons;
    let subtitle = item.subtitle.as_str();
    let (has_username, has_uri) = match &item.r#type {
        CipherListViewType::Login(login) => (
            login.username.is_some(),
            login
                .uris
                .as_ref()
                .and_then(|u| u.first())
                .and_then(|u| u.uri.as_deref())
                .is_some(),
        ),
        _ => (false, false),
    };

    let icon: Element<'a, ItemListMessage, AppTheme> = if !show_favicons {
        initial_circle(&item.name, colors)
    } else {
        match &item.r#type {
            CipherListViewType::Login(login) => {
                let resolved = login
                    .uris
                    .as_ref()
                    .and_then(|u| u.first())
                    .and_then(|u| u.uri.as_deref())
                    .and_then(favicon::hostname_for_fetch)
                    .map(|h| favicon.get(active_user, &h));
                match resolved {
                    // `Handle` clones share the same `Id`, so iced's GPU
                    // texture cache hits for the lifetime of the session.
                    // `get()` also triggers a fetch on the first sight of
                    // this hostname in the viewport.
                    Some(IconState::Found(handle)) => png_icon(handle.clone()),
                    // Pending / Missing / no URI / non-fetchable URI → globe.
                    _ => png_icon(favicon::globe_handle()),
                }
            }
            // TODO: replace with per-type BWI icons (Card/Identity/Note/SshKey).
            _ => png_icon(favicon::globe_handle()),
        }
    };

    let info = column![
        text(&item.name)
            .size(14)
            .color(colors.text_primary)
            .wrapping(Wrapping::None)
            .ellipsis(Ellipsis::End),
        text(subtitle)
            .size(14)
            .color(colors.text_secondary)
            .wrapping(Wrapping::None)
            .ellipsis(Ellipsis::End),
    ]
    .spacing(2)
    .width(Fill);

    let mut actions: Vec<Element<'a, ItemListMessage, AppTheme>> = Vec::new();
    if has_uri {
        actions.push(action_icon(
            icons::BOX_ARROW_UP_RIGHT,
            ItemListMessage::OpenExternal(i),
            colors,
        ));
    }
    if has_username {
        actions.push(action_icon(
            icons::COPY,
            ItemListMessage::CopyUsername(i),
            colors,
        ));
    }
    actions.push(action_icon(
        icons::THREE_DOTS_VERTICAL,
        ItemListMessage::MoreOptions(i),
        colors,
    ));

    let actions_row = row(actions).spacing(2).align_y(Alignment::Center);

    let content = row![icon, info, actions_row]
        .spacing(12)
        .align_y(Alignment::Center);

    // The button itself is 48 tall (32 icon + 8+8 padding) and the
    // separator is 1 tall, summing to ROW_HEIGHT. `virtual_list` enforces
    // the uniform height on the outer wrapper — we only care about the
    // inner content and horizontal padding here.
    let button = buttons::ghost(
        content,
        is_selected,
        colors.item_hover,
        colors.item_hover,
        RADIUS_MD,
    )
    .on_press(ItemListMessage::ItemSelected(i))
    .padding([8, 8])
    .width(Fill);

    container(column![button, components::separator_h()].width(Fill))
        .padding([0, 16])
        .into()
}

fn action_icon<'a>(
    icon: icons::Icon,
    message: ItemListMessage,
    colors: &AppColors,
) -> Element<'a, ItemListMessage, AppTheme> {
    buttons::ghost_icon(icon.render(14.0, colors.text_secondary), colors.item_hover)
        .on_press(message)
        .padding([4, 6])
        .into()
}

/// Deterministic-colour initial-letter circle used when `show_favicons` is
/// off — preserves the legacy visual for users who prefer it.
fn initial_circle<'a>(name: &str, colors: &'a AppColors) -> Element<'a, ItemListMessage, AppTheme> {
    let initial = name
        .chars()
        .next()
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let circle_color = initial_color(name);
    container(text(initial).size(14).color(colors.text_primary))
        .width(32)
        .height(32)
        .center_x(32)
        .center_y(32)
        .style(move |_theme: &AppTheme| {
            container::Style::default()
                .background(circle_color)
                .border(iced::border::rounded(16))
        })
        .into()
}

/// Render a pre-clipped icon in a 32×32 slot. The image has the rounded-rect
/// alpha mask baked in by [`crate::favicon`], so iced just blits the decoded
/// texture — no container-level clipping required (iced doesn't support it
/// anyway).
fn png_icon<'a>(handle: image::Handle) -> Element<'a, ItemListMessage, AppTheme> {
    container(image::Image::new(handle).width(32).height(32))
        .width(32)
        .height(32)
        // `RADIUS_SM` exists purely so future style closures that want a
        // subtle surround (focus ring, selected outline) can reference the
        // same constant as the baked-in alpha mask.
        .style(|_theme: &AppTheme| {
            container::Style::default().border(iced::border::rounded(RADIUS_SM))
        })
        .into()
}

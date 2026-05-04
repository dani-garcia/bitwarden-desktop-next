//! Native muda menu (macOS app menu, optionally Win/Linux). Builds the OS-
//! level menu tree from the [`super::definitions::MENUS`] table; the OS owns
//! the resulting `Menu` for the lifetime of the process.

use std::collections::HashMap;

use muda::{Menu, MenuItem as MudaMenuItem, PredefinedMenuItem, Submenu};

use super::{
    EnabledWhen, MenuAction, MenuEntry, MenuState, definitions::MENUS, should_use_native_title_bar,
};

pub struct NativeMenuHandle {
    pub actions: HashMap<muda::MenuId, MenuAction>,
    pub items: Vec<(MudaMenuItem, EnabledWhen)>,
}

impl NativeMenuHandle {
    /// Resolve a muda `MenuId` to a `MenuAction` if it belongs to the native
    /// app menu. Returns `None` for unrelated ids (e.g. tray menu events).
    /// The caller drains `muda::MenuEvent::receiver()` — a global singleton
    /// shared with the tray menu — and dispatches by id.
    pub fn resolve(&self, id: &muda::MenuId) -> Option<MenuAction> {
        self.actions.get(id).copied()
    }

    pub fn sync_enabled(&self, state: &MenuState) {
        for (item, when) in &self.items {
            item.set_enabled(when.check(state));
        }
    }
}

/// Returns `None` if native menu is not enabled.
pub fn attach_menu(raw_id: u64) -> Option<NativeMenuHandle> {
    if !should_use_native_title_bar() {
        return None;
    }

    let mut actions = HashMap::new();
    let mut items = Vec::new();

    let menu = Menu::new();
    for (label_key, entries) in MENUS {
        let label = crate::services::i18n::lookup(label_key);
        let submenu = Submenu::new(format!("&{label}"), true);
        append_entries_to_submenu(&submenu, entries, &mut actions, &mut items);
        let _ = menu.append(&submenu);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = raw_id;
        menu.init_for_nsapp();
    }
    #[cfg(target_os = "windows")]
    {
        unsafe {
            let _ = menu.init_for_hwnd(raw_id as isize);
        }
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        // `should_use_native_title_bar()` returns false on Linux so the
        // early return above fires first; reaching here would mean someone
        // forced the native path on an unsupported platform.
        let _ = raw_id;
        let _ = menu;
        return None;
    }

    // The `Menu` and its `MenuItem`s must outlive the process — muda's
    // `init_for_{nsapp,hwnd}` hands ownership to the OS, which holds the
    // references until shutdown. Dropping the `Menu` would leave the OS
    // with a dangling pointer. Leaking is the idiomatic way to hand
    // ownership off to the platform menu system.
    std::mem::forget(menu);

    Some(NativeMenuHandle { actions, items })
}

fn append_entries_to_submenu(
    submenu: &Submenu,
    entries: &[MenuEntry],
    actions: &mut HashMap<muda::MenuId, MenuAction>,
    items: &mut Vec<(MudaMenuItem, EnabledWhen)>,
) {
    let menu_items: Vec<Box<dyn muda::IsMenuItem>> = entries
        .iter()
        .map(|entry| -> Box<dyn muda::IsMenuItem> {
            if entry.is_separator() {
                Box::new(PredefinedMenuItem::separator())
            } else if !entry.children.is_empty() {
                let sub = Submenu::new(entry.display_label(), true);
                append_entries_to_submenu(&sub, entry.children, actions, items);
                Box::new(sub)
            } else {
                let accel = entry.shortcut.and_then(|s| s.to_accelerator());
                let item = MudaMenuItem::new(entry.display_label(), true, accel);
                if let Some(action) = entry.action {
                    actions.insert(item.id().clone(), action);
                }
                items.push((item.clone(), entry.enabled));
                Box::new(item)
            }
        })
        .collect();
    let refs: Vec<&dyn muda::IsMenuItem> = menu_items.iter().map(|b| b.as_ref()).collect();
    let _ = submenu.append_items(&refs);
}

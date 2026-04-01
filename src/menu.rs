use muda::{Menu, MenuItem, PredefinedMenuItem, Submenu};

/// Build the menu and attach it to the window using its raw platform handle.
/// On Windows, `raw_id` is the HWND. On macOS, it attaches to the NSApp.
pub fn attach_menu(raw_id: u64) {
    let menu = build_menu();

    #[cfg(target_os = "windows")]
    unsafe {
        let _ = menu.init_for_hwnd(raw_id as isize);
    }

    #[cfg(target_os = "macos")]
    {
        let _ = raw_id;
        menu.init_for_nsapp();
    }

    // Leak the menu so it lives for the entire process
    // (muda menus must outlive the window they're attached to)
    std::mem::forget(menu);
}

fn build_menu() -> Menu {
    let menu = Menu::new();

    // File
    let file_menu = Submenu::new("&File", true);
    let _ = file_menu.append_items(&[
        &MenuItem::new("Add New Login", true, None),
        &MenuItem::new("Add New Item", true, None),
        &MenuItem::new("Add New Folder", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Sync Vault", true, None),
        &MenuItem::new("Import Vault", true, None),
        &MenuItem::new("Export Vault", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Settings", true, None),
        &MenuItem::new("Lock", true, None),
        &MenuItem::new("Lock All Accounts", true, None),
        &MenuItem::new("Log Out", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Quit Bitwarden", true, None),
    ]);

    // Edit
    let edit_menu = Submenu::new("&Edit", true);
    let _ = edit_menu.append_items(&[
        &PredefinedMenuItem::undo(None),
        &PredefinedMenuItem::redo(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::cut(None),
        &PredefinedMenuItem::copy(None),
        &PredefinedMenuItem::paste(None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::select_all(None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Copy Username", true, None),
        &MenuItem::new("Copy Password", true, None),
        &MenuItem::new("Copy Verification Code (TOTP)", true, None),
    ]);

    // View
    let view_menu = Submenu::new("&View", true);
    let _ = view_menu.append_items(&[
        &MenuItem::new("Search Vault", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Generator", true, None),
        &MenuItem::new("Password History", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Zoom In", true, None),
        &MenuItem::new("Zoom Out", true, None),
        &MenuItem::new("Reset Zoom", true, None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::fullscreen(None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Reload", true, None),
    ]);

    // Account
    let account_menu = Submenu::new("&Account", true);
    let _ = account_menu.append_items(&[
        &MenuItem::new("Premium Membership", true, None),
        &MenuItem::new("Change Master Password", true, None),
        &MenuItem::new("Two-step Login", true, None),
        &MenuItem::new("Fingerprint Phrase", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Delete Account", true, None),
    ]);

    // Window
    let window_menu = Submenu::new("&Window", true);
    let _ = window_menu.append_items(&[
        &PredefinedMenuItem::minimize(None),
        &MenuItem::new("Hide to Menu Bar", true, None),
        &MenuItem::new("Always on Top", true, None),
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::close_window(None),
    ]);

    // Help
    let help_menu = Submenu::new("&Help", true);
    let _ = help_menu.append_items(&[
        &MenuItem::new("Help & Feedback", true, None),
        &MenuItem::new("File a Bug Report", true, None),
        &MenuItem::new("Legal", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Follow Us", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Go to Web Vault", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("Get Mobile App", true, None),
        &MenuItem::new("Get Browser Extension", true, None),
        &PredefinedMenuItem::separator(),
        &MenuItem::new("About Bitwarden", true, None),
    ]);

    let _ = menu.append_items(&[
        &file_menu,
        &edit_menu,
        &view_menu,
        &account_menu,
        &window_menu,
        &help_menu,
    ]);

    menu
}

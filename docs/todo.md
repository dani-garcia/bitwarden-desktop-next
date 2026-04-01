# TODO

## Next Up

- **Tray icon** — Use `tray-icon` crate (sister to `muda`, same raw window handle approach). Should show Bitwarden shield icon, right-click context menu with Lock/Quit.
- **Executable icon** — Embed `.ico` in the Windows executable via `winresource` build script. Use the Bitwarden shield icon. Also set the window icon via iced's `window::Settings::icon`.

## UI Polish

- Vault screen needs iteration (hasn't been refined as much as login screen)
- Sidebar: indented tree hierarchy (Vault > All vaults > My vault)
- Search bar: magnifying glass icon prefix
- Wire native menu items to actual app actions (lock, quit, etc.)
- Light mode / theme system (refactor `theme.rs` from constants to a Theme struct)

## Refactoring

- **Component extraction** — Reusable styled components for buttons (primary, secondary/outline), text (heading, body, caption sizes), input fields (with floating label). Currently button styles are duplicated inline in each view with hardcoded colors and sizes. Extract into shared widget helpers or style functions in `widgets/` so views just call e.g. `primary_button("Unlock", Message::Unlock)`.
- **Theme system** — Refactor `theme.rs` from module-level `const` colors to a `Theme` struct with variants (Dark, Light). All widgets should pull colors from the active theme rather than importing constants directly. This is a prerequisite for light mode.
- **Color tokens** — Define semantic color roles (e.g. `bg_primary`, `bg_surface`, `fg_brand`, `border_default`) rather than using raw hex everywhere. This makes theme switching and consistency easier.

## Functionality

- Wire menu "Quit" to actually close the app
- Wire menu "Lock" / "Lock All" to lock user sessions
- Handle tray icon click to show/hide window
- "Unlock with Windows Hello" button (future, needs keyring/biometric integration)

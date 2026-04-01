# UI Changelog

Tracks visual changes across iterations. Screenshots are in `img/` under versioned folders.

## Version 1 (initial)

**Screenshots**: `img/version1/`

First working stub UI with both screens functional.

- Login screen: centered card, password input, unlock/logout buttons, account switcher
- Vault screen: sidebar with category filters, scrollable item list, search bar
- Dark theme applied throughout
- Account switcher with multi-user support

**Known issues vs original**:
- Password toggle ("Show") was a separate button outside the input field
- No colored initial circles on vault items
- No divider between sidebar and item list
- No "bitwarden" header in sidebar
- Buttons had 4px border-radius (original uses ~12px)

## Version 2

**Screenshots**: `img/version2/`

**Changes from v1**:
- Password visibility toggle (◎ icon) moved inside the input field border
- Colored initial circles added to each vault item (deterministic color from item name hash)
- Vertical divider line between sidebar and item list
- "bitwarden / Password Manager" header added to sidebar top
- Items display from top of list

## Version 3 (current)

**Screenshots**: `img/version3/`

**Changes from v2**:
- All colors updated to match official Bitwarden dark theme exactly (sourced from `variables.scss` and `tw-theme.css`)
  - Background: `#1f242e`, Header/Sidebar: `#1d293d`, Card: `#2f343d`, Border: `#4c525f`
  - Text secondary: `#bac0ce`, Primary accent (brand-400): `#6baefa`
- Button border-radius updated from 4px to **12px** (matches `tw-rounded-xl` from component library)
- Unlock button: now uses brand-400 `#6baefa` fill with dark text, matching the official primary button
- Log out button: uses `#272b32` background with `#4c525f` border, matching `buttonBackgroundColor`/`buttonBorderColor`
- Top bar now uses distinct header background color (`#1d293d`) separate from main background
- Bitwarden logo SVG added to top-left of login screen (from `clients/apps/web/src/images/logo-white.svg`)
- Added `svg` feature to iced for SVG rendering support
- Logo stored in `assets/logo-white.svg`

**Remaining differences vs original**:
- Original's Unlock button appears slightly darker blue (may be using legacy theme `#175ddc` instead of brand-400)
- Sidebar lacks indented tree hierarchy (Vault > All vaults > My vault)
- No lock icon above "Your vault is locked" title
- No decorative background illustrations on login screen (bottom corners)
- Missing magnifying glass icon in search bar
- Account switcher missing server URL and dropdown arrow
- Colors are still module-level constants, not a switchable theme system

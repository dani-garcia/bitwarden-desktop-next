# Design Reference

This document captures findings from the official Bitwarden desktop app (`clients/` submodule) that inform the native Rust UI.

## Reference App Location

The official app is at `clients/` (git submodule of https://github.com/bitwarden/clients). The desktop app is in `clients/apps/desktop/`, but many UI components are shared in `clients/libs/`.

## Button Styles

The app has two style systems. The **newer component library** (Tailwind-based) is what the current app uses:

### New Component Library (active)
- **Source**: `clients/libs/components/src/button/button.component.ts`
- **Border radius**: `tw-rounded-xl` = **12px** (we use 20px for a more pill-shaped look matching the actual rendered app)
- **Default button padding**: `9px 16px` (vertical/horizontal)
- **Font size**: 14px (0.875rem)
- **Border width**: 1px

### Old Desktop SCSS (legacy, being phased out)
- **Source**: `clients/apps/desktop/src/scss/buttons.scss`
- **Border radius**: `$border-radius` = **3px**
- **Padding**: `7px 15px`

### Current Implementation
| Property | Primary (Unlock) | Secondary (Log out) |
|----------|------------------|---------------------|
| Border radius | 20px | 20px |
| Background | `#6baefa` (transparent on hover: `#aac3ef`) | transparent (hover: `#1f2a3c`) |
| Text color | dark (`#070b18`) | accent blue (`#6baefa`) |
| Border | same as bg | accent blue (`#6baefa`) |

## Logo

### Files Found
- **White SVG**: `clients/apps/web/src/images/logo-white.svg` (290x45 viewbox, white fill)
- **Desktop PNG (white)**: `clients/apps/desktop/src/images/logo-white@2x.png`
- **Icon components**: `clients/libs/assets/src/svg/svgs/bitwarden-logo.icon.ts`

### Current Implementation
Logo SVG copied to `assets/logo-white.svg` and rendered via iced's `svg` widget at 209x35.

## Lock Screen Icon

- **Source**: `clients/libs/assets/src/svg/svgs/lock.icon.ts`
- Extracted to `assets/lock-icon.svg` with dark theme colors baked in
- Rendered at 64x60 above the "Your vault is locked" title

## Login Page Background Illustrations

- **Left**: `clients/libs/assets/src/svg/svgs/background-left-illustration.ts`
- **Right**: `clients/libs/assets/src/svg/svgs/background-right-illustration.ts`
- Extracted to `assets/bg-left.svg` and `assets/bg-right.svg` with dark theme fill colors
- Rendered at bottom corners, fixed size, 11% opacity

### Illustration Colors (dark theme)
From `clients/libs/components/src/tw-theme.css` (dark mode section):
- `tw-fill-illustration-outline` → `rgb(23, 93, 220)` = `#175ddc`
- `tw-fill-illustration-bg-primary` → `rgb(170, 195, 239)` = `#aac3ef`
- `tw-fill-illustration-bg-secondary` → `rgb(121, 161, 233)` = `#79a1e9`
- `tw-fill-illustration-bg-tertiary` → `rgb(243, 246, 249)` = `#f3f6f9`
- `tw-fill-illustration-logo` → `rgb(255, 255, 255)` = `#ffffff`
- `tw-fill-illustration-tertiary` → `rgb(255, 191, 0)` = `#ffbf00` (gold stars on lock)

## Color Palette (Dark Theme)

**Important**: The SCSS values in `variables.scss` do NOT match the actual rendered colors. Always verify with a color picker against the running app.

### Actual Rendered Colors (picked from running app)
| Role | Hex | Source |
|------|-----|--------|
| Main window background | `#070b18` | Color picker |
| Header / account bar | `#1e2939` | Color picker |
| Card background | `#101828` | Color picker |
| Sidebar background | `#1d293d` | `--color-nav-bg-primary` (gray-800) |
| Input/button hover | `#1f2a3c` | Color picker |
| Border | `#4c525f` | `boxBorderColor` in SCSS |
| Primary accent | `#6baefa` | `--color-brand-400` (dark mode) |
| Accent hover | `#aac3ef` | Color picker |
| Text primary | `#ffffff` | |
| Text secondary | `#bac0ce` | `mutedColor` in SCSS |

### SCSS Values (for reference, may not match rendered)
From `clients/apps/desktop/src/scss/variables.scss` (dark theme map):
- `backgroundColor`: `#1f242e`
- `backgroundColorAlt2`: `#15181e` (used as window bg in `window.main.ts`)
- `boxBackgroundColor`: `#2f343d`
- `boxBorderColor`: `#4c525f`
- `buttonBackgroundColor`: `#272b32`
- `primaryColor` / `buttonPrimaryColor`: `#6f9df1`

## Font

- **Font family**: Inter (`$font-family-sans-serif` in `variables.scss`)
- **Source**: https://github.com/rsms/inter (bundled as `assets/InterVariable.ttf`)
- **Weight**: Normal (400) for body, Bold for "bit" in logo text

## Account Switcher

### Original Behavior (see `img/original/Account.png`)
- Shows initials circle + email + server URL + up/down arrow
- Dropdown pops under the bar as a floating panel
- Dropdown shows other accounts (not the active one) + "+ Add account"

### Current Implementation
- Trigger: initials + email + server + arrow (⌃/⌄)
- Dropdown: floats via `stack!` at the page level (not inside the top bar)
- Excludes active account, includes "+ Add account"

## Native Menu Bar

### Source
Menu entries from `clients/apps/desktop/src/main/menu/`:
- `menu.file.ts` — Add items, sync/import/export, settings, lock, quit
- `menu.edit.ts` — Undo/redo, cut/copy/paste, copy username/password/TOTP
- `menu.view.ts` — Search, generator, zoom, fullscreen, reload
- `menu.account.ts` — Premium, change password, 2FA, fingerprint, delete
- `menu.window.ts` — Minimize, hide to menu bar, always on top, close
- `menu.help.ts` — Help, bug report, legal, follow us, web vault, mobile/browser, about
- `menu.bitwarden.ts` — macOS-only app menu (prepended on macOS)

### Current Implementation
- Uses `muda` crate for native OS menus
- All menu items are stubs (no event handling wired yet)
- Attached via `Window::Opened` subscription → `raw_id()` → `init_for_hwnd()`

## Screens

### Login/Lock Screen
- Top bar: account switcher (right), bitwarden logo below on main background (left)
- Lock icon (shield with stars) centered
- "Your vault is locked" + email
- Card: floating label input, Unlock button, "or", Log out button
- Background illustrations at bottom corners (11% opacity)
- "Accessing bitwarden.com" status text at bottom

### Main Vault Screen
- **Top bar**: Search input + account switcher
- **Sidebar**: "bitwarden / Password Manager" header, category tree navigation
- **Item list**: Scrollable, colored initial circles + name + subtitle
- **Detail pane** (future): Item details on selection

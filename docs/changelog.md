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

**Changes from v2** (many iterations within this version):

### Colors
- Colors updated to match actual rendered app (picked with color picker, not from SCSS):
  - Main background: `#070b18`, Header bar: `#1e2939`, Card: `#101828`
  - Border: `#4c525f`, Text secondary: `#bac0ce`, Primary accent: `#6baefa`
- Note: SCSS values in `variables.scss` differ from actual rendered colors. Always verify with a color picker.
- Input field and Log out button backgrounds set to transparent (match card bg)

### Login Screen Layout
- Lock icon SVG added above "Your vault is locked" (extracted from `clients/libs/assets/src/svg/svgs/lock.icon.ts`)
- Title and email moved above the card (card only contains input + buttons)
- Floating label "Master password (required)" on the input border (intersecting the card border line)
- Decorative background illustrations at bottom corners at 11% opacity (from `clients/libs/assets/src/svg/svgs/background-{left,right}-illustration.ts`)
- "Accessing bitwarden.com" status text centered at the bottom
- Card pinned near top with fixed spacing (doesn't move on resize)

### Buttons
- Border-radius increased to 20px (more pill-shaped, matching original)
- Unlock button: `#6baefa` fill, dark text, hover `#aac3ef`
- Log out button: transparent bg, accent blue text and border, hover bg `#1f2a3c`
- Show/hide toggle hover: `#1f2a3c`

### Logo & Branding
- Bitwarden logo SVG in top-left of main window (from `clients/apps/web/src/images/logo-white.svg`)
- Logo stored in `assets/logo-white.svg`

### Account Switcher
- Now shows email + server URL (e.g. "bitwarden.com") + up/down arrow
- Dropdown floats over content via `stack!` (doesn't push the top bar down)
- Dropdown excludes the currently active account, shows "+ Add account"
- Server URL added to `AccountEntry` and `UserSession` types

### Fonts
- Inter font loaded from `assets/InterVariable.ttf` (matches official Bitwarden app)
- Font sizes bumped +2 across the board for better readability

### Menu Bar
- macOS: native menu via `muda` crate (`init_for_nsapp()`)
- Windows/Linux: custom-drawn menu bar inside iced window (avoids 1px transparent gap from native Win32 menus)
- Full menu structure: File, Edit, View, Account, Window, Help
- Menu entries with platform-specific keyboard shortcuts (Ctrl on Win/Linux, Cmd on macOS)
- Submenus open on hover, render as separate panel beside the parent dropdown
- `MenuState` struct controls enabled/disabled per item (`!isLocked`, `hasAccounts`, etc.)
- Menu items are stubs (no functionality wired yet)

### Icons
- Bootstrap Icons v1.13.1 integrated via TTF font
- `build.rs` auto-generates `Icon` constants from the Bootstrap Icons CSS
- `Icon` type with `.render(size, color)` method for type-safe usage
- Used in sidebar (category icons), search bar (magnifying glass), password toggle (eye), account switcher (chevrons), menu submenus (chevron-right)

### Infrastructure
- `DEV_SCREEN=vault` env var to skip to vault screen without recompilation
- `DEV_SCREENSHOT=path.png` captures via iced's `window::screenshot()` API and exits
- `screenshot-all.ps1` automates login + vault screenshots (no OS-level window capture)
- Window title changed to "Bitwarden [Next]" to avoid conflict with real Bitwarden app
- Default window size 1080x720, min 600x480
- Window background color set via `iced::theme::Style` to prevent transparent artifacts
- Console hidden in release builds (`windows_subsystem = "windows"`)
- Release profile: LTO, single codegen unit, strip, opt-level "s", panic abort

### Assets added
- `assets/logo-white.svg` — Bitwarden wordmark + shield
- `assets/lock-icon.svg` — Lock screen icon (shield with stars)
- `assets/bg-left.svg` — Left background illustration
- `assets/bg-right.svg` — Right background illustration
- `assets/InterVariable.ttf` — Inter variable font
- `assets/bootstrap-icons-1.13.1.ttf` — Bootstrap Icons font
- `assets/bootstrap-icons-1.13.1.css` — Bootstrap Icons CSS (for build.rs code generation)

**Remaining differences vs original**:
- Sidebar lacks indented tree hierarchy (Vault > All vaults > My vault)
- Colors are still module-level constants, not a switchable theme system (no light mode)
- No tray icon
- No executable/window icon
- Menu items are non-functional stubs (no actions wired)
- Menu submenus for Lock vault / Log out should be dynamically populated with account emails
- Vault screen hasn't been iterated on as much as login screen

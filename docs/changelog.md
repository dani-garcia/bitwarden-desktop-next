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

## Version 4 (current)

**Screenshots**: `img/version4/`

**Changes from v3** — 2025 desktop redesign iteration targeting `designs/Desktop 2025/Windows.png`.

### Sidebar Overhaul
- Bitwarden Icons (bwi) font integrated (`assets/bwi-font.ttf`) — sidebar now uses official BWI icons (vault, send, generate, import, download, user, star, login, credit-card, identity, note, key, archive, trash)
- Combined logo SVG (`assets/password-manager-logo.svg`) — shield + "bitwarden" + "Password Manager" in one SVG, extracted from `clients/libs/assets/src/svg/svgs/password-manager.ts`
- Expanded sidebar width: 232px
- All sidebar text made brighter (white instead of secondary gray)
- Section header text: 16px, nav item text: 15px, icons: 17-18px
- Section chevrons: 21px BWI angle-up/down icons
- Collapse/expand caret: 32px BWI angle-left/right icons
- Selected item background: `#121a27` (subtle dark highlight, doesn't reach sidebar edges)
- All items have 6px rounded corners and are inset from sidebar edges (8px horizontal padding)
- Horizontal separator line above the collapse button
- Bottom padding on collapsed rail so toggle doesn't touch window edge

### Content Area Corner Radius
- Content area has 10px top-left border radius creating a smooth curve where it meets the sidebar
- Achieved by giving the main row `HEADER_BG` background and content area `BACKGROUND` with rounded corner

### Detail Pane (NEW)
- Right-side detail pane shows selected item information
- Three sections with cards: "Item details" (name), "Login credentials" (username + copy, password + eye/copy), "Autofill options" (website + copy + open URL)
- Header bar: "View login/card/identity/note/SSH key" title + close (X) button
- Bottom bar: blue pill "Edit" button (left) + red trash icon (right)
- Vertical separator line on left edge spanning full height
- Detail pane background: `#121a27`, cards and header: `#202733`
- Section labels: 14px white text
- Field labels: 11px muted text, values: 14px white
- Action buttons use BWI icons: copy, eye, external-link, close, edit, trash

### Resizable Panes (PaneGrid)
- Item list and detail pane use `iced::widget::pane_grid` for draggable resize
- Initial split: 40% list, 60% detail
- Minimum pane size: 250px
- When no item selected, list fills the full content area (no PaneGrid)
- PaneGrid state persists across view rebuilds

### Stable Item Selection
- Items now have a stable `id` field (not just list index)
- Detail pane stays open when searching/filtering — looks up item by ID from all vault items
- Search no longer clears selection
- Text input has stable ID (`vault-search`) + focus task to prevent focus loss during PaneGrid rebuilds

### Color Updates
- Background: `#202733` (left pane, cards, right pane header)
- Sidebar/title bar: `#303946`
- Detail pane background: `#121a27`
- Separators/borders: `#303946`
- Button primary: `#65abff` (was `#175ddc`)
- Button text (New/Edit): `#121a27` (dark on bright blue)
- Muted text: `#8898b5` (was `#6e788a`)
- Login page: window background `#121a27`, card `#202733` (swapped from vault)

### Item List Improvements
- Horizontal separators between items with 16px horizontal padding (don't reach edges)
- Header separator also has 16px padding
- Selected item uses hover color (`#3c424e`) with 6px rounded corners, inset from edges
- Scrollbar styled: transparent rail background, only grab handle visible (`ITEM_HOVER` color)

### Antialiasing
- `antialiasing(true)` enabled on the app builder

### Code Cleanup
- Shared UI helpers extracted to `src/widgets/common.rs`: `separator_h()`, `separator_v()`, `hover_button_style()`, `styled_card()`
- Theme constants deduplicated, inline colors extracted (`BUTTON_PRIMARY_HOVER`, `BUTTON_HOVER_SUBTLE`)
- Border radius constants added: `RADIUS_SM` (4), `RADIUS_MD` (6), `RADIUS_LG` (8), `RADIUS_PILL` (20)
- Unused theme constants removed (`INPUT_BG`, `SELECTED_BG`, `SIDEBAR_BG`, `ICON_RAIL_BG`, `ICON_RAIL_ACTIVE`)
- Imports consolidated: grouped `use crate::{a, b}`, nested widget imports, fully qualified paths replaced with direct imports
- `Padding::from([x.0, y.0])` simplified to `[x, y]` across all files

### Assets Added
- `assets/bwi-font.ttf` — Bitwarden Icons font (from `clients/libs/angular/src/scss/bwicons/fonts/`)
- `assets/password-manager-logo.svg` — Combined shield + wordmark + subtitle logo
- `assets/bitwarden-shield.svg` — Shield icon for collapsed rail

**Remaining differences vs original**:
- Sidebar lacks indented tree hierarchy (Vault > All vaults > My vault)
- Colors are still module-level constants, not a switchable theme system (no light mode)
- No tray icon or executable/window icon
- Menu items are non-functional stubs
- Copy/edit/delete buttons are stubs (no clipboard or persistence)
- No TOTP circular timer (placeholder only)
- Sidebar expand/collapse not animated

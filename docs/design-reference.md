# Design Reference

This document captures findings from the official Bitwarden desktop app (`clients/` submodule) that inform the native Rust UI.

## Reference App Location

The official app is at `clients/` (git submodule of https://github.com/bitwarden/clients). The desktop app is in `clients/apps/desktop/`, but many UI components are shared in `clients/libs/`.

## Button Styles

The app has two style systems. The **newer component library** (Tailwind-based) is what the current app uses:

### New Component Library (active)
- **Source**: `clients/libs/components/src/button/button.component.ts`
- **Border radius**: `tw-rounded-xl` = **12px**
- **Default button padding**: `9px 16px` (vertical/horizontal)
- **Font size**: 14px (0.875rem)
- **Border width**: 1px (implied by `calc(0.625rem - 1px)` padding adjustment)

### Old Desktop SCSS (legacy, being phased out)
- **Source**: `clients/apps/desktop/src/scss/buttons.scss`
- **Border radius**: `$border-radius` = **3px** (from `variables.scss:13`)
- **Padding**: `7px 15px`
- **Border**: `1px solid`
- **Font size**: `$font-size-base` (14px)

### Current App Values (what we should match)

Based on visual inspection of the original screenshots and the newer component library:

| Property | Primary (Unlock) | Secondary (Log out) |
|----------|------------------|---------------------|
| Border radius | 12px | 12px |
| Border width | 0px (filled) | 1px |
| Padding | ~9px 16px | ~9px 16px |
| Font size | 14px | 14px |

**Status (v3)**: Updated to match — 12px radius, correct colors, correct border widths.

## Logo

### Files Found
- **White SVG**: `clients/apps/web/src/images/logo-white.svg` (290x45 viewbox, white fill)
- **Desktop PNG (white)**: `clients/apps/desktop/src/images/logo-white@2x.png`
- **Desktop PNG (dark)**: `clients/apps/desktop/src/images/logo-dark@2x.png`
- **Icon components**: `clients/libs/assets/src/svg/svgs/bitwarden-logo.icon.ts`, `bitwarden-icon.ts`

The logo SVG contains both the shield icon and the "bitwarden" wordmark. The shield is the left portion of the SVG path.

### Usage in Our App
Currently we render "bitwarden" as styled text in the sidebar. For better fidelity, we should either:
1. Embed the SVG as an iced `Image` or `Svg` widget (iced supports SVG via the `svg` feature)
2. Continue with text but match the font weight (bold "bit" + regular "warden")

## Login Page Background

### Decorative Illustrations
- **Left illustration**: `clients/libs/assets/src/svg/svgs/background-left-illustration.ts`
- **Right illustration**: `clients/libs/assets/src/svg/svgs/background-right-illustration.ts`
- **Layout component**: `clients/libs/components/src/landing-layout/landing-layout.component.ts`

These are inline SVG components (TypeScript, not standalone files). They render gear/shield decorative shapes at the bottom corners.

**Styling**:
- Positioned at bottom-left and bottom-right
- Width: 35%, max-width: 450px
- **Opacity: 11%** (`tw-opacity-[.11]`)
- Colors use themed fill classes: `tw-fill-illustration-bg-primary`, `tw-fill-illustration-bg-secondary`, etc.

### Implementation Plan
Extract the SVG path data from the TypeScript files, save as standalone `.svg` files, and render them in the login view using iced's SVG support (requires `svg` feature on iced crate).

## Color Palette (Dark Theme)

### Official Theme Values
From `clients/apps/desktop/src/scss/variables.scss` and the Tailwind config:

| Role | Our Current | Notes |
|------|-------------|-------|
| Background | `#171e2b` | Close to original |
| Sidebar BG | `#1a2332` | Close to original |
| Card BG | `#212b3c` | Close to original |
| Accent/Primary | `#175ddc` | Official brand blue |
| Text Primary | `#ffffff` | Correct |
| Text Secondary | `#8b95a5` | Close |
| Border | `#2c3544` | Close |

**Note**: Colors will need to be refactored from module-level constants to a theme struct/trait system to support light/dark mode switching.

## Account Switcher

### Original Behavior
- Shows user avatar (initials in colored circle) + email
- Shows server URL below email (e.g., "bitwarden.com")
- Has a dropdown arrow (▼) indicator
- Dropdown lists all accounts with locked/unlocked status

### Our Current State
- Shows initial circle + email
- Missing: server URL display
- Missing: dropdown arrow indicator
- `AccountEntry` struct needs a `server_url: String` field added

## Screens

### Login/Lock Screen
- Centered card on dark background with decorative illustrations (bottom corners)
- Lock icon above "Your vault is locked" title (we don't have this yet)
- Email display (read-only)
- Password input with visibility toggle (eye icon inside input)
- "Unlock" primary button
- "Unlock with Windows Hello" secondary button (optional, future)
- "Log out" button
- Account switcher in top-right

### Main Vault Screen
- **Top bar**: Search input (with magnifying glass icon) + account switcher
- **Sidebar**: Logo header, collapsible tree navigation with icons
  - Vault section (All vaults, My vault, shared vaults — tree hierarchy)
  - Types section (Favorites, Login, Card, Identity, Secure Note, SSH Key)
  - Separator
  - Archive, Trash
  - Collections, Folders
  - Send, Generator
- **Item list**: Scrollable, each item has colored initial circle + name + subtitle
- **Detail pane** (future): Shows item details on selection

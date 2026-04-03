# Decisions

## Framework: Iced 0.14

**Decision**: Use Iced as the GUI framework.

**Alternatives considered**: Dioxus (webview-based desktop mode, custom renderer still maturing), Slint (GPL/commercial license concern).

**Rationale**: Pure Rust, wgpu rendering, no webview, Elm architecture ensures repaints only on state change, ~25k GitHub stars, actively maintained, cross-platform.

## Crate Name: bitwarden-desktop-next

**Decision**: Named `bitwarden-desktop-next` to avoid conflict with existing "bitwarden-lite" project.

## UI Only (No Business Logic)

**Decision**: This app is stub UI only. All crypto, API, authentication, and vault operations will come from a separate SDK (`PasswordManagerClient`).

**Rationale**: Separation of concerns. The SDK handles security-sensitive operations; this project handles presentation.

## Multi-User State Model

**Decision**: App state uses `HashMap<UserId, UserSession>` mirroring the SDK's `HashMap<UserId, PasswordManagerClient>`.

**Rationale**: Makes future SDK integration a clean swap. Multiple accounts can be available simultaneously with one active at a time.

## Colors: Picker Over SCSS

**Decision**: Use color-picked values from the actual running Bitwarden app, not SCSS/CSS variable values.

**Rationale**: The SCSS values in `variables.scss` and Tailwind CSS vars don't match what the app actually renders. For example, `backgroundColor` is `#1f242e` in SCSS but the actual window background is `#070b18`. The SCSS has three background levels (`backgroundColor`, `backgroundColorAlt`, `backgroundColorAlt2`) and it's unclear which applies where. Always verify with a color picker (Win+Shift+C with PowerToys).

## Menus: Native on macOS, Custom-Drawn Elsewhere

**Decision**: Use `muda` for native menus on macOS only. Draw the menu bar inside iced on Windows and Linux.

**Previous approach**: Used `muda::init_for_hwnd()` on Windows for native Win32 menus.

**Why changed**: Native Win32 menus create a 1px transparent gap between the menu bar and the iced client area (DWM compositor border). This is a known muda issue (#123, fixed for Tauri's webview but not for raw wgpu rendering). The dark theme (`MenuTheme::Dark`) made the entire menu bar transparent instead of dark. Drawing the menu ourselves eliminates the gap entirely.

**macOS**: Keeps native menus via `init_for_nsapp()` because macOS users expect the system menu bar at the top of the screen, and it integrates properly there.

**Linux**: Uses the custom-drawn menu (same as Windows) to avoid GTK dependencies.

## Icons: Bootstrap Icons (Self-Bundled)

**Decision**: Bundle Bootstrap Icons TTF font directly and auto-generate Rust constants via `build.rs`.

**Alternatives considered**: `iced_fonts` crate (adds a dependency for something trivial), `iced_aw` icons (claimed to have an Icon enum but doesn't), Font Awesome, Nerd Fonts, individual SVGs per icon.

**Rationale**: Bootstrap Icons is what the official Bitwarden app uses (`bwi-*` prefixed classes). The crates that wrap icon fonts are trivially simple — they just bundle a TTF and map names to Unicode codepoints. Doing it ourselves means zero extra dependencies and full control over the version. The `build.rs` script parses the official CSS file and generates an `Icon` type with `.render()` method.

**WOFF→TTF conversion**: The Bootstrap Icons release only ships WOFF/WOFF2, but iced's `fontdb` only supports TTF/OTF. We used the `wuff` crate (pure Rust WOFF decoder) once to convert the WOFF to TTF, saved the result in `assets/`, then removed `wuff` from dependencies.

## Font: Inter

**Decision**: Bundle the Inter variable font (`InterVariable.ttf`) and set it as the iced default font.

**Rationale**: The official Bitwarden app uses Inter (`$font-family-sans-serif: Inter` in `variables.scss`). Inter is heavier than the default system font, which gives the UI the same visual weight as the original.

## Button Style: Pill-Shaped (20px radius)

**Decision**: Use 20px border-radius for buttons (more pill-shaped than the 12px from the component library).

**Rationale**: Visual inspection of the actual rendered app shows more rounded buttons than the Tailwind `tw-rounded-xl` (12px) suggests. 20px matches the actual look better.

## Overlays: Legacy stack! Approach (Replaced)

**Previous decision**: Used `iced::widget::stack!` at the page level for floating elements.

**Status**: Replaced by DropDown widget (see "Overlays: DropDown Widget" section below). Stack approach had limitations: couldn't cover the title bar, required manual backdrops, fragile offset calculations.

## Window Title

**Decision**: Use "Bitwarden [Next]" as the window title.

**Rationale**: Avoids conflict with the real Bitwarden app when running side-by-side for visual comparison. The PowerShell screenshot script used `FindWindow` by title, which would match the wrong window otherwise.

## Theme System (Implemented)

**Decision**: Custom `AppTheme` struct implementing `iced::theme::Base` + widget `Catalog` traits. Dark/light palettes in separate files.

**Status**: Complete. Runtime toggle via Help > About Bitwarden. Light palette is placeholder.

**Rationale**: iced 0.14's `application()` is generic over Theme. Custom theme gives `&AppTheme` in all `.style()` closures with direct access to 20+ semantic color tokens. No thread_local or registry needed.

## Overlays: DropDown Widget (replaces stack! pattern)

**Decision**: Use a local fork of iced_aw's `DropDown` widget (`components/drop_down.rs`) with custom alignment variants (`BelowLeft`, `BelowRight`, `AboveRight`).

**Previous approach**: `iced::widget::stack!` with manual backdrops for click-outside-to-close.

**Why changed**: Stack-based overlays couldn't cover the title bar (they're inside the page layout). DropDown uses iced's native overlay system which renders at the window level, handles click-outside automatically, and positions relative to the trigger widget.

## State Decentralization (Implemented)

**Decision**: Each view (LoginView, VaultView, TitleBarState) owns its state and has its own `update()` method returning action enums.

**Status**: Complete. App struct down from 30 to 11 fields.

**Rationale**: Different views will be owned by different teams. Event handling should be close to the components that initiate events. Action enums handle cross-cutting effects cleanly.

## Component Library

**Decision**: Button convenience constructors (`buttons::primary(content)`) that take `impl Into<Element>` and return `Button` with style pre-applied. Style functions are internal.

**Rationale**: Callers never need to write `.style()` closures for common button variants. The `impl Into<Element>` signature matches iced's `button()` for familiarity.

## Native Menu Unification

**Decision**: Single `MENUS` definition drives both custom and native menus. `NativeMenuHandle` (on App, not static) bridges events.

**Rationale**: Avoids duplicating menu structure. `Shortcut::to_accelerator()` reuses existing `display()` for muda compatibility. `DEV_BOTH_MENUS=1` shows both simultaneously for comparison.

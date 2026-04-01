# Decisions

## Framework: Iced 0.14

**Decision**: Use Iced as the GUI framework.

**Alternatives considered**: Dioxus (webview-based desktop mode, custom renderer still maturing), Slint (GPL/commercial license concern).

**Rationale**: Pure Rust, wgpu rendering, no webview, Elm architecture ensures repaints only on state change, ~25k GitHub stars, actively maintained, cross-platform.

## Crate Name: bitwarden-desktop-native

**Decision**: Named `bitwarden-desktop-native` to avoid conflict with existing "bitwarden-lite" project.

## UI Only (No Business Logic)

**Decision**: This app is stub UI only. All crypto, API, authentication, and vault operations will come from a separate SDK (`PasswordManagerClient`).

**Rationale**: Separation of concerns. The SDK handles security-sensitive operations; this project handles presentation.

## Multi-User State Model

**Decision**: App state uses `HashMap<UserId, UserSession>` mirroring the SDK's `HashMap<UserId, PasswordManagerClient>`.

**Rationale**: Makes future SDK integration a clean swap. Multiple accounts can be available simultaneously with one active at a time.

## DEV_SCREEN Environment Variable

**Decision**: Use `DEV_SCREEN` env var (not compile-time feature flags) to skip to specific screens during development.

**Rationale**: Avoids recompilation when screenshotting different views. `DEV_SCREEN=vault cargo run` skips directly to vault.

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

## Screenshots: iced Built-in Over OS-Level Capture

**Decision**: Use `iced::window::screenshot()` instead of PowerShell window capture.

**Previous approach**: `screenshot.ps1` used `FindWindow` + `CopyFromScreen` to capture the window by title.

**Why changed**: OS-level capture had reliability issues — wrong window captured when another app shared the title ("Bitwarden"), focus problems, DPI scaling mismatches. The iced screenshot API captures directly from the renderer, is cross-platform, and pixel-perfect.

**Implementation**: Set `DEV_SCREENSHOT=path.png` env var. The app captures on first render (via `Window::Opened` subscription) and exits. The `screenshot-all.ps1` script runs the app twice with different `DEV_SCREEN`/`DEV_SCREENSHOT` values.

## Overlays via stack! (No Native Popover)

**Decision**: Use `iced::widget::stack!` at the page level for floating elements (dropdowns, menus).

**Rationale**: Iced doesn't have native popover/overlay support. Putting dropdowns inside the widgets that trigger them (e.g. inside the account switcher or menu bar) causes them to expand the parent container rather than floating over content. The solution is to render the trigger inside its container and the dropdown in a `stack!` layer at the top of the view, with spacers for positioning.

## Window Title

**Decision**: Use "Bitwarden [Next]" as the window title.

**Rationale**: Avoids conflict with the real Bitwarden app when running side-by-side for visual comparison. The PowerShell screenshot script used `FindWindow` by title, which would match the wrong window otherwise.

## Theme System (TODO)

**Decision**: Refactor from module-level color constants (`theme.rs`) to a trait-based or struct-based theme system.

**Status**: Not yet implemented. Currently all colors are `const` in `theme.rs`.

**Rationale**: The app needs to support light/dark mode and potentially custom themes. A `Theme` struct with variants would allow runtime switching.

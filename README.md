# bitwarden-desktop-next

A vibe-coded project investigating how the [Bitwarden Rust SDK](https://github.com/bitwarden/sdk-internal) can be used to build a native desktop password manager — a slim alternative to the official Electron client, written entirely in Rust on top of [iced 0.15](https://iced.rs/).

The UI is hand-rolled; everything below it (crypto, auth, vault decrypt, sync) goes through the real `bitwarden-*` SDK crates. Treat this as an exploration, not a production app.

See [docs/architecture.md](docs/architecture.md) for the project layout and [docs/decisions.md](docs/decisions.md) for the rationale behind the major choices.

## Running it

```bash
cargo run -p fake-data                        # Regenerate per-user SQLite + mock.json under data/
cargo run                                     # Loading screen → login

cargo run --bin packager                      # Package .app/.dmg/.msi/.deb/.rpm
```

Useful dev modes:

```bash
DEV_BOTH_MENUS=1 cargo run                                            # Native + custom menus side-by-side
RUST_LOG=bitwarden_desktop_next=debug,bitwarden_core=debug cargo run  # Full SDK tracing
```

First-run tip: the real login command isn't wired yet (see TODO below). Run `cargo run -p fake-data` first to seed a local SQLite vault you can unlock.

### Seeded accounts

`fake-data` writes three users to `data/`. None of these are real — passwords are hardcoded and ship in the dev binary, so don't reuse them anywhere.

| Email                   | Password   | Notes                                                              |
| ----------------------- | ---------- | ------------------------------------------------------------------ |
| `alice@example.com`     | `password` | Personal vault. Master password + biometrics unlock methods.       |
| `alice@acmecorp.com`    | `123456`   | Work vault with orgs + collections. Master password + PIN unlock.  |
| `loadtest@example.com`  | `loadtest` | ~20k mixed ciphers for layout / scroll / filter perf testing.      |

## Supported

- Multi-user vault unlock against a local SDK-backed SQLite store
- Cipher list / detail / edit / delete
- Send list / detail / create (in-memory)
- Password generator
- Magnify quick launcher (`Ctrl+Shift+Space` / `Cmd+Shift+Space`)
- System tray + single-instance wake-up
- Custom title-bar menus (Windows/Linux) and native `muda` menus (macOS)
- Light / dark theme, persisted settings (`data/settings.json`)
- Favicons in the vault list
- Packaging to `.app` / `.dmg` / `.msi` / `.deb` / `.rpm`

## TODO (highlights)

Full list lives in [docs/todo.md](docs/todo.md). The big ones:

- **Real login command** — current flow only unlocks an existing local DB seeded by `fake-data`
- **2FA / Registration / SSO / master-password-hint** screens
- **PIN unlock** and **biometrics** (Touch ID / Windows Hello / polkit)
- **SSH agent** — parked until upstream V2 ships
- **Browser integration**, **autotype**, native-messaging host
- **Send**: swap in-memory map for the SDK's `Repository<Send>`, add file sends
- **Right-click context menus** on text inputs
- Many settings tabs are UI-only stubs that emit a "not supported yet" toast

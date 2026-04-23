//! Platform + stateful cross-view services.
//!
//! Each subfolder is one service, constructed once in `App::new` and consumed
//! by views via `UpdateCtx`. Services may not import from `views/`, `components/`,
//! or `app/`.

pub mod clipboard;
pub mod favicon;
pub mod i18n;
pub mod instance_lock;
pub mod menu;
pub mod preferences;
pub mod sdk;
pub mod settings;
pub mod tray;

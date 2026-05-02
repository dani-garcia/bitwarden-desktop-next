//! Finds Fluent (`.ftl`) message IDs that are no longer referenced from the
//! Rust source.
//!
//! `i18n-embed-fl` validates Rust → `.ftl` references at compile time, but
//! unused keys in the `.ftl` itself are silent — the file just grows. This
//! tool walks the workspace's Rust sources for `fl!("<id>")` calls and
//! diffs the set against the IDs declared in
//! `assets/i18n/en/bitwarden_desktop_next.ftl` (the canonical English
//! catalogue). Run from the workspace root with:
//!
//! ```bash
//! cargo run -p i18n-unused
//! ```
//!
//! Exit code is `0` if everything is referenced, `1` if there are any
//! unused keys (so it slots into CI). The non-English `.ftl` files aren't
//! checked — they should mirror the English file's key set, and a
//! diff-against-English drift check is the next obvious extension.

use std::{
    collections::BTreeSet,
    env,
    fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const CATALOGUE: &str = "assets/i18n/en/bitwarden_desktop_next.ftl";
const SOURCE_ROOTS: &[&str] = &["crates", "bitwarden_license", "tools"];

/// Keys that are referenced indirectly and won't show up in a `fl!(...)` or
/// `E(...)` scan. Adding to this list is preferable to deleting a key that
/// is in fact in use somewhere the linter can't see.
const ALLOW_EXACT: &[&str] = &[
    // Top-level menu titles in `services::menu::MENUS` are bare string
    // literals in a `(&str, &[MenuEntry])` tuple, not wrapped in `E(...)`.
    "menu-file",
    "menu-edit",
    "menu-view",
    "menu-window",
    "menu-help",
    "menu-account",
];

/// Prefixes for dynamic-key lookups. Keys matching any prefix below stay
/// flagged as referenced even when no exact-string scan finds them.
const ALLOW_PREFIX: &[&str] = &[
    // `services::i18n::language_native_name(tag)` builds the key as
    // `format!("language-name-{tag}")`.
    "language-name-",
];

fn main() -> ExitCode {
    let workspace_root = match find_workspace_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let catalogue_path = workspace_root.join(CATALOGUE);
    let declared = match parse_ftl_keys(&catalogue_path) {
        Ok(set) => set,
        Err(e) => {
            eprintln!("error: failed to read {}: {e}", catalogue_path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut referenced: BTreeSet<String> = BTreeSet::new();
    for root in SOURCE_ROOTS {
        let path = workspace_root.join(root);
        if !path.is_dir() {
            continue;
        }
        if let Err(e) = collect_fl_references(&path, &mut referenced) {
            eprintln!("error: walking {}: {e}", path.display());
            return ExitCode::FAILURE;
        }
    }

    let unused: Vec<&String> = declared
        .difference(&referenced)
        .filter(|k| {
            !ALLOW_EXACT.contains(&k.as_str())
                && !ALLOW_PREFIX.iter().any(|p| k.starts_with(p))
        })
        .collect();

    // We deliberately don't flag "referenced but undeclared" — the `fl!`
    // macro fails the build on an unknown id, so anything in that diff is
    // a false positive from comments / docstrings / non-`fl!` string
    // literals that happen to look like the pattern.

    if unused.is_empty() {
        println!(
            "All {} keys in {} are referenced.",
            declared.len(),
            CATALOGUE
        );
        return ExitCode::SUCCESS;
    }

    println!("Unused keys in {} ({}):", CATALOGUE, unused.len());
    for k in &unused {
        println!("  {k}");
    }

    ExitCode::FAILURE
}

/// Walk up from `cwd` looking for a `Cargo.toml` containing `[workspace]`.
/// Lets the tool work whether `cargo run` is invoked from the workspace
/// root or from inside `tools/i18n-unused`.
fn find_workspace_root() -> Result<PathBuf, String> {
    let mut current = env::current_dir().map_err(|e| format!("getting cwd: {e}"))?;
    loop {
        let candidate = current.join("Cargo.toml");
        if candidate.is_file() {
            let content = fs::read_to_string(&candidate)
                .map_err(|e| format!("reading {}: {e}", candidate.display()))?;
            if content.contains("[workspace]") {
                return Ok(current);
            }
        }
        if !current.pop() {
            return Err("could not find workspace root (no [workspace] Cargo.toml above cwd)".into());
        }
    }
}

/// Parse top-level Fluent message identifiers from a `.ftl` file. Format:
///
/// ```text
/// message-id = some value
/// ```
///
/// Indented lines belong to the previous message (multi-line value or
/// attribute) and are ignored. Lines starting with `#` are comments. Term
/// definitions (`-term-id =`) are also skipped — they're internal to
/// Fluent and not addressed via `fl!`.
fn parse_ftl_keys(path: &Path) -> Result<BTreeSet<String>, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut keys = BTreeSet::new();
    for line in content.lines() {
        let bytes = line.as_bytes();
        // Indented continuation, blank, comment, or term — skip.
        match bytes.first() {
            None | Some(b' ') | Some(b'\t') | Some(b'#') | Some(b'-') => continue,
            _ => {}
        }
        let Some(eq_idx) = line.find('=') else {
            continue;
        };
        let key = line[..eq_idx].trim();
        if !key.is_empty() && key.chars().all(is_fluent_id_char) {
            keys.insert(key.to_string());
        }
    }
    Ok(keys)
}

fn is_fluent_id_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '-' || c == '_'
}

/// Recursively scan `root` for Rust source files and collect message IDs
/// out of `fl!("...")` calls. Skips `target/`. Doesn't try to be a Rust
/// parser — `fl!` is always invoked with a string literal so a substring
/// scan is enough.
fn collect_fl_references(root: &Path, out: &mut BTreeSet<String>) -> Result<(), String> {
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
        for entry in entries {
            let entry = entry.map_err(|e| e.to_string())?;
            let path = entry.path();
            if path.file_name().is_some_and(|n| n == "target") {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let src = fs::read_to_string(&path)
                    .map_err(|e| format!("reading {}: {e}", path.display()))?;
                extract_string_arg(&src, b"fl!(", out);
                // Menu service: `E("menu-...")` builds a `MenuEntry` whose
                // label is resolved at render time via `i18n::lookup`. The
                // sibling helper `L("...")` is for *literal* labels (brand
                // names) that bypass i18n — those must NOT be counted, and
                // an exact-match needle of `E(` won't pick them up.
                extract_string_arg(&src, b"E(", out);
            }
        }
    }
    Ok(())
}

/// Pull the first string-literal argument out of every call matching
/// `<needle>"..."` in a Rust source string. Used to scrape both `fl!(...)`
/// and the menu service's `E(...)` constructor — both of which take a
/// Fluent message ID as their first argument.
fn extract_string_arg(src: &str, needle: &[u8], out: &mut BTreeSet<String>) {
    let bytes = src.as_bytes();
    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if &bytes[i..i + needle.len()] != needle {
            i += 1;
            continue;
        }
        // Reject identifier-prefix collisions: `E(` matched as the
        // *suffix* of `Some(`, `Type(`, etc. Real call sites have either
        // start-of-input or a non-ident char immediately before the
        // needle.
        if i > 0 {
            let prev = bytes[i - 1];
            if prev.is_ascii_alphanumeric() || prev == b'_' {
                i += 1;
                continue;
            }
        }
        // Skip whitespace after `fl!(` so we accept both `fl!("x")` and
        // `fl!( "x" )`.
        let mut j = i + needle.len();
        while j < bytes.len() && bytes[j].is_ascii_whitespace() {
            j += 1;
        }
        if j >= bytes.len() || bytes[j] != b'"' {
            i += 1;
            continue;
        }
        // Walk to the closing `"` — the `fl!` macro doesn't accept escapes
        // in the message id slot, but be defensive against `\"` anyway.
        let mut k = j + 1;
        let mut id = String::new();
        while k < bytes.len() {
            match bytes[k] {
                b'\\' if k + 1 < bytes.len() => {
                    // Skip escaped character.
                    k += 2;
                }
                b'"' => break,
                c => {
                    id.push(c as char);
                    k += 1;
                }
            }
        }
        if k < bytes.len() && bytes[k] == b'"' && !id.is_empty() {
            out.insert(id);
            i = k + 1;
        } else {
            i = j + 1;
        }
    }
}

//! Lints the workspace's Fluent (`.ftl`) catalogues.
//!
//! `i18n-embed-fl` validates Rust → `.ftl` references at compile time, but
//! everything else about the catalogue files is invisible to `cargo build`:
//! unused keys, duplicate definitions in the same file, and per-locale
//! drift against the canonical catalogue. This tool runs three passes:
//!
//! 1. **Duplicate keys** in any `.ftl` we look at.
//! 2. **Unused keys** — message IDs declared in the canonical English
//!    catalogue but never referenced from the workspace's Rust sources via
//!    `fl!("<id>")` or `E("<id>")`.
//! 3. **Locale parity** — for every non-English locale under
//!    `assets/i18n/<locale>/bitwarden_desktop_next.ftl`, list keys missing
//!    relative to English (untranslated) and stale keys present only in
//!    that locale.
//!
//! Run from the workspace root with:
//!
//! ```bash
//! cargo run -p i18n-unused
//! ```
//!
//! Any finding fails CI (exit code `1`); a clean run prints a one-liner
//! and exits `0`.
//!
//! Note: the "referenced but undeclared" diff is deliberately not reported
//! — `fl!` fails the build on an unknown id, so anything in that diff is a
//! false positive from comments / docstrings / non-`fl!` string literals
//! that happen to look like the pattern.

use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    process::ExitCode,
};

const I18N_ROOT: &str = "assets/i18n";
const CATALOGUE_LOCALE: &str = "en";
const CATALOGUE_FILE: &str = "bitwarden_desktop_next.ftl";
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
    // Endonym key, declared by every locale and resolved by
    // `services::i18n::language_label` through a per-locale
    // `FluentLanguageLoader` rather than the active loader.
    "language-name-self",
];

/// Prefixes for dynamic-key lookups. Keys matching any prefix below stay
/// flagged as referenced even when no exact-string scan finds them.
const ALLOW_PREFIX: &[&str] = &[];

/// Parsed view of a single `.ftl` file.
struct Ftl {
    /// Unique top-level message IDs declared in the file.
    keys: BTreeSet<String>,
    /// IDs that were declared more than once, with the (1-based) line
    /// numbers of every occurrence in source order.
    duplicates: BTreeMap<String, Vec<usize>>,
}

fn main() -> ExitCode {
    let workspace_root = match find_workspace_root() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let i18n_root = workspace_root.join(I18N_ROOT);
    let catalogue_path = i18n_root.join(CATALOGUE_LOCALE).join(CATALOGUE_FILE);

    let canonical = match parse_ftl(&catalogue_path) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: failed to read {}: {e}", catalogue_path.display());
            return ExitCode::FAILURE;
        }
    };

    let mut had_errors = false;
    let mut sections_printed = 0usize;

    // 1. Duplicates in the canonical catalogue.
    if !canonical.duplicates.is_empty() {
        if sections_printed > 0 {
            println!();
        }
        report_duplicates(&catalogue_path, &canonical.duplicates);
        had_errors = true;
        sections_printed += 1;
    }

    // 2. Unused keys (canonical only — the build fails on missing
    //    translations in other locales, so they're not the source of truth).
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
    let unused: Vec<&String> = canonical
        .keys
        .difference(&referenced)
        .filter(|k| {
            !ALLOW_EXACT.contains(&k.as_str()) && !ALLOW_PREFIX.iter().any(|p| k.starts_with(p))
        })
        .collect();
    if !unused.is_empty() {
        if sections_printed > 0 {
            println!();
        }
        println!(
            "Unused keys in {}/{}/{} ({}):",
            I18N_ROOT,
            CATALOGUE_LOCALE,
            CATALOGUE_FILE,
            unused.len()
        );
        for k in &unused {
            println!("  {k}");
        }
        had_errors = true;
        sections_printed += 1;
    }

    // 3. Per-locale duplicates + parity.
    let other_locales = match discover_other_locales(&i18n_root) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error: enumerating locales under {}: {e}",
                i18n_root.display()
            );
            return ExitCode::FAILURE;
        }
    };
    for locale in &other_locales {
        let locale_path = i18n_root.join(locale).join(CATALOGUE_FILE);
        if !locale_path.is_file() {
            continue;
        }
        let parsed = match parse_ftl(&locale_path) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("error: failed to read {}: {e}", locale_path.display());
                return ExitCode::FAILURE;
            }
        };
        if !parsed.duplicates.is_empty() {
            if sections_printed > 0 {
                println!();
            }
            report_duplicates(&locale_path, &parsed.duplicates);
            had_errors = true;
            sections_printed += 1;
        }
        let missing: Vec<&String> = canonical.keys.difference(&parsed.keys).collect();
        let stale: Vec<&String> = parsed.keys.difference(&canonical.keys).collect();
        if !missing.is_empty() || !stale.is_empty() {
            if sections_printed > 0 {
                println!();
            }
            report_parity(&locale_path, &missing, &stale);
            had_errors = true;
            sections_printed += 1;
        }
    }

    if had_errors {
        return ExitCode::FAILURE;
    }

    if sections_printed == 0 {
        println!(
            "All {} keys in {}/{}/{} are referenced; locale catalogues match.",
            canonical.keys.len(),
            I18N_ROOT,
            CATALOGUE_LOCALE,
            CATALOGUE_FILE,
        );
    }
    ExitCode::SUCCESS
}

fn report_duplicates(path: &Path, duplicates: &BTreeMap<String, Vec<usize>>) {
    println!(
        "Duplicate keys in {} ({}):",
        path.display(),
        duplicates.len()
    );
    for (key, lines) in duplicates {
        let lines_str = lines
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        println!("  {key} (lines {lines_str})");
    }
}

fn report_parity(path: &Path, missing: &[&String], stale: &[&String]) {
    println!(
        "Locale parity {}: {} missing, {} stale",
        path.display(),
        missing.len(),
        stale.len(),
    );
    if !missing.is_empty() {
        println!("  missing (defined in {CATALOGUE_LOCALE}, absent here):");
        for k in missing {
            println!("    {k}");
        }
    }
    if !stale.is_empty() {
        println!("  stale (defined here, absent in {CATALOGUE_LOCALE}):");
        for k in stale {
            println!("    {k}");
        }
    }
}

/// List every locale directory under `i18n_root` that isn't the canonical
/// catalogue locale, sorted for stable output.
fn discover_other_locales(i18n_root: &Path) -> Result<Vec<String>, String> {
    if !i18n_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut out = Vec::new();
    for entry in fs::read_dir(i18n_root).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.path().is_dir() {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
            continue;
        };
        if name == CATALOGUE_LOCALE {
            continue;
        }
        out.push(name);
    }
    out.sort();
    Ok(out)
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
            return Err(
                "could not find workspace root (no [workspace] Cargo.toml above cwd)".into(),
            );
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
///
/// Records every occurrence so duplicates can be reported with line
/// numbers; the deduped set lives in `Ftl::keys`.
fn parse_ftl(path: &Path) -> Result<Ftl, String> {
    let content = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut counts: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (idx, line) in content.lines().enumerate() {
        let bytes = line.as_bytes();
        match bytes.first() {
            None | Some(b' ') | Some(b'\t') | Some(b'#') | Some(b'-') => continue,
            _ => {}
        }
        let Some(eq_idx) = line.find('=') else {
            continue;
        };
        let key = line[..eq_idx].trim();
        if key.is_empty() || !key.chars().all(is_fluent_id_char) {
            continue;
        }
        counts.entry(key.to_string()).or_default().push(idx + 1);
    }
    let mut keys = BTreeSet::new();
    let mut duplicates = BTreeMap::new();
    for (key, lines) in counts {
        keys.insert(key.clone());
        if lines.len() > 1 {
            duplicates.insert(key, lines);
        }
    }
    Ok(Ftl { keys, duplicates })
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

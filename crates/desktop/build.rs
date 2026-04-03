use std::{collections::BTreeMap, path::Path};

const FONT_VERSION: &str = "1.13.1";

fn main() {
    generate_bootstrap_icons();
}

fn generate_bootstrap_icons() {
    let css_file = format!("../../assets/bootstrap-icons-{FONT_VERSION}.css");
    let ttf_file = format!("../../assets/bootstrap-icons-{FONT_VERSION}.ttf");

    println!("cargo:rerun-if-changed={css_file}");
    println!("cargo:rerun-if-changed={ttf_file}");
    println!("cargo:rerun-if-changed=build.rs");

    let css = std::fs::read_to_string(&css_file)
        .unwrap_or_else(|e| panic!("Failed to read {css_file}: {e}"));

    let mut icons = BTreeMap::new();

    for line in css.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(".bi-")
            && let Some((name, rest)) = rest.split_once("::before")
            && let Some(start) = rest.find("\"\\")
        {
            let hex_start = start + 2;
            if let Some(end) = rest[hex_start..].find('"') {
                let hex = &rest[hex_start..hex_start + end];
                let mut const_name = name.to_uppercase().replace('-', "_");
                if const_name.starts_with(|c: char| c.is_ascii_digit()) {
                    const_name = format!("_{const_name}");
                }
                icons.insert(const_name, hex.to_string());
            }
        }
    }

    let out_dir = std::env::var("OUT_DIR").unwrap();
    let gen_path = Path::new(&out_dir).join("bootstrap_icons_generated.rs");

    let mut code = String::new();
    code.push_str(&format!(
        "// Auto-generated from bootstrap-icons-{FONT_VERSION} — do not edit\n\n"
    ));
    code.push_str(&format!(
        "pub const FONT_BYTES: &[u8] = include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/../../assets/bootstrap-icons-{FONT_VERSION}.ttf\"));\n\n"
    ));

    for (name, hex) in &icons {
        code.push_str(&format!("pub const {name}: Icon = Icon('\\u{{{hex}}}');\n"));
    }

    code.push_str(&format!("pub const ICON_COUNT: usize = {};\n", icons.len()));

    std::fs::write(&gen_path, code).expect("Failed to write generated icons");
}

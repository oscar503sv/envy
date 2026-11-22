//! Guards ADR-002: the core must not depend on GTK.

#![allow(
    clippy::string_slice,
    reason = "indices come from match_indices on ASCII patterns, so they are char boundaries"
)]

use std::fs;
use std::path::{Path, PathBuf};

const GTK_ROOTS: &[&str] = &["gtk", "gtk4", "gdk", "gdk4", "adw", "libadwaita"];
const DOMAIN_ROOTS: &[&str] = &[
    "gtk",
    "gtk4",
    "gdk",
    "gdk4",
    "adw",
    "libadwaita",
    "gio",
    "glib",
];

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn code_part(line: &str) -> Option<&str> {
    let trimmed = line.trim_start();
    if trimmed.starts_with("//") {
        return None;
    }
    Some(line.split("//").next().unwrap_or(line))
}

fn line_violates(line: &str, roots: &[&str]) -> bool {
    let Some(code) = code_part(line) else {
        return false;
    };
    roots.iter().any(|root| {
        let use_pat = format!("use {root}");
        let has_use = code.match_indices(&use_pat).any(|(i, _)| {
            let before_ok = code[..i]
                .chars()
                .next_back()
                .is_none_or(|c| !is_ident_char(c));
            let after = code[i + use_pat.len()..].chars().next();
            before_ok && matches!(after, Some(':' | ';' | ' ' | '{'))
        });
        let path_pat = format!("{root}::");
        let has_path = code.match_indices(&path_pat).any(|(i, _)| {
            // A leading `::` is an absolute crate path (`::gtk::Window`), but
            // `foo::gtk::` is a submodule of another crate.
            let prefix = code[..i].strip_suffix("::").unwrap_or(&code[..i]);
            prefix
                .chars()
                .next_back()
                .is_none_or(|c| !is_ident_char(c) && c != ':')
        });
        let has_extern = code.contains(&format!("extern crate {root}"));
        has_use || has_path || has_extern
    })
}

fn collect_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries = fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot read {dir:?}: {e}"));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot read entry in {dir:?}: {e}"))
            .path();
        if path.is_dir() {
            collect_rs_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

fn violations_in(subdir: &str, roots: &[&str]) -> Vec<String> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs_files(&base.join(subdir), &mut files);
    assert!(!files.is_empty(), "no .rs files found under {subdir}");

    let mut found = Vec::new();
    for file in files {
        let content =
            fs::read_to_string(&file).unwrap_or_else(|e| panic!("cannot read {file:?}: {e}"));
        for (idx, line) in content.lines().enumerate() {
            if line_violates(line, roots) {
                let rel = file.strip_prefix(base).unwrap_or(&file);
                found.push(format!("{}:{}: {}", rel.display(), idx + 1, line.trim()));
            }
        }
    }
    found
}

#[test]
fn domain_has_no_gtk_glib_or_gio() {
    let found = violations_in("src/domain", DOMAIN_ROOTS);
    assert!(
        found.is_empty(),
        "forbidden dependencies in domain:\n{}",
        found.join("\n")
    );
}

#[test]
fn services_have_no_gtk() {
    let found = violations_in("src/services", GTK_ROOTS);
    assert!(
        found.is_empty(),
        "forbidden dependencies in services:\n{}",
        found.join("\n")
    );
}

#[test]
fn detector_flags_forbidden_usages() {
    assert!(line_violates("use gtk::prelude::*;", DOMAIN_ROOTS));
    assert!(line_violates(
        "let x = glib::MainContext::default();",
        DOMAIN_ROOTS
    ));
    assert!(line_violates("extern crate gio;", DOMAIN_ROOTS));
    assert!(line_violates("use adw::{self, prelude::*};", GTK_ROOTS));
    assert!(line_violates("use ::gtk::Window;", GTK_ROOTS));
    assert!(line_violates(
        "let c = ::glib::MainContext::default();",
        DOMAIN_ROOTS
    ));
    assert!(!line_violates("use gio::File;", GTK_ROOTS));
}

#[test]
fn detector_ignores_comments_and_lookalikes() {
    assert!(!line_violates("// uses gtk::Window", DOMAIN_ROOTS));
    assert!(!line_violates("/// see adw::Dialog", DOMAIN_ROOTS));
    assert!(!line_violates("//! use gtk::prelude::*;", DOMAIN_ROOTS));
    assert!(!line_violates("let my_gtk = 1;", DOMAIN_ROOTS));
    assert!(!line_violates("my_gtk::f()", DOMAIN_ROOTS));
    assert!(!line_violates("foo::gtk::f()", DOMAIN_ROOTS));
    assert!(!line_violates("let a = 1; // gtk::Window", DOMAIN_ROOTS));
}

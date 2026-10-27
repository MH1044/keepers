//! Source hygiene: fails the build when a file about to be published contains something that
//! would identify a person or a machine, or something shaped like a secret.
//!
//! The specification is `docs/TESTING.md` §8. This file lives in a crate's `tests/` directory
//! because a root `tests/` folder is never compiled in a virtual workspace; it walks up from
//! `CARGO_MANIFEST_DIR` to the repository root. It names no person and no address, so it needs
//! no edit when a contributor changes.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;

/// Set to about half the current file count, so an empty walk cannot pass vacuously.
const MIN_FILES: usize = 45;

/// Text is decided by extension, never by sniffing, so a new binary fixture cannot opt itself in.
const TEXT_EXTENSIONS: &[&str] = &[
    "rs", "ts", "tsx", "mts", "cts", "js", "jsx", "mjs", "cjs", "py", "toml", "json", "jsonc",
    "yml", "yaml", "md", "html", "css", "txt", "sh", "ps1", "xml", "svg", "csv", "tsv", "lock",
];

/// Upstream metadata neither written nor controlled here: exempt from the email rule only.
const EMAIL_EXEMPT_FILES: &[&str] = &["package-lock.json", "Cargo.lock", "THIRD_PARTY.md"];

/// The one allowed token is the Publik app token, public by design, in exactly this file.
const APP_TOKEN_FILE: &str = "crates/keepers-publik/publik-app-token.txt";

/// Directories skipped when git is unavailable (git's own listing already excludes them).
const SKIPPED_DIRS: &[&str] = &[
    ".git",
    "node_modules",
    "target",
    "dist",
    "build",
    "out",
    "coverage",
    "release",
    ".venv",
    "__pycache__",
];

/// Root-level directories skipped when git is unavailable: downloaded models and private shoots.
const SKIPPED_ROOT_DIRS: &[&str] = &["models", "eval-data"];

fn repository_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    loop {
        if dir.join(".git").exists() || dir.join("docs").join("TESTING.md").exists() {
            return dir;
        }
        assert!(
            dir.pop(),
            "the repository root (a directory with docs/TESTING.md) was not found above the crate"
        );
    }
}

/// The files that could be published: tracked plus untracked-but-not-ignored, as git sees them.
fn published_files(root: &Path) -> Vec<PathBuf> {
    let listing = Command::new("git")
        .args([
            "ls-files",
            "-z",
            "--cached",
            "--others",
            "--exclude-standard",
        ])
        .current_dir(root)
        .output();
    match listing {
        Ok(output) if output.status.success() => output
            .stdout
            .split(|b| *b == 0)
            .filter(|entry| !entry.is_empty())
            .map(|entry| root.join(String::from_utf8_lossy(entry).as_ref()))
            .filter(|path| path.is_file())
            .collect(),
        _ => walk_without_git(root),
    }
}

fn walk_without_git(root: &Path) -> Vec<PathBuf> {
    let root_owned = root.to_path_buf();
    ignore::WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_exclude(true)
        .filter_entry(move |entry| {
            let name = entry.file_name().to_string_lossy();
            if SKIPPED_DIRS.contains(&name.as_ref()) {
                return false;
            }
            let at_root = entry.path().parent() == Some(root_owned.as_path());
            !(at_root && SKIPPED_ROOT_DIRS.contains(&name.as_ref()))
        })
        .build()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_some_and(|t| t.is_file()))
        .map(|entry| entry.into_path())
        .collect()
}

fn is_text(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if name.starts_with("LICENSE") || name.starts_with("COPYING") {
        return true;
    }
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| TEXT_EXTENSIONS.contains(&ext.to_ascii_lowercase().as_str()))
}

struct TextFile {
    relative: String,
    bytes: Vec<u8>,
}

fn text_files() -> Vec<TextFile> {
    let root = repository_root();
    let mut files: Vec<TextFile> = published_files(&root)
        .into_iter()
        .filter(|path| is_text(path))
        .map(|path| {
            let relative = path
                .strip_prefix(&root)
                .unwrap_or(&path)
                .to_string_lossy()
                .replace('\\', "/");
            let bytes = fs::read(&path).unwrap_or_else(|e| panic!("cannot read {relative}: {e}"));
            TextFile { relative, bytes }
        })
        .collect();
    files.sort_by(|a, b| a.relative.cmp(&b.relative));
    files
}

/// Every (file, line number, line) in the tree, as text.
fn lines(files: &[TextFile]) -> impl Iterator<Item = (&str, usize, String)> {
    files.iter().flat_map(|file| {
        let text = String::from_utf8_lossy(&file.bytes).into_owned();
        text.lines()
            .enumerate()
            .map(|(i, line)| (file.relative.as_str(), i + 1, line.to_string()))
            .collect::<Vec<_>>()
    })
}

fn report(what: &str, remedy: &str, hits: &[String]) {
    assert!(
        hits.is_empty(),
        "{what} in {} place(s); {remedy}:\n  {}",
        hits.len(),
        hits.join("\n  ")
    );
}

#[test]
fn walk_finds_the_tree() {
    let files = text_files();
    assert!(
        files.len() > MIN_FILES,
        "only {} text files were found; the walk did not see the repository",
        files.len()
    );
}

#[test]
fn no_control_characters() {
    let mut hits = Vec::new();
    for file in text_files() {
        let mut line = 1;
        for &byte in &file.bytes {
            if byte == b'\n' {
                line += 1;
            } else if (byte < 0x20 && byte != b'\t' && byte != b'\r') || byte == 0x7F {
                hits.push(format!("{}:{line}: byte 0x{byte:02X}", file.relative));
                break;
            }
        }
    }
    report(
        "control characters other than tab, LF and CR",
        "write the character as an escape sequence",
        &hits,
    );
}

#[test]
fn no_home_directory_paths() {
    // A drive letter, slashes (doubled when JSON-escaped), the users directory and a name; or the
    // Linux and macOS home prefixes followed by a name. Described here rather than shown.
    let home = Regex::new(r"(?:[A-Za-z]:[\\/]+Users[\\/]+|/home/|/Users/)[A-Za-z0-9._-]+").unwrap();
    let files = text_files();
    let hits: Vec<String> = lines(&files)
        .filter(|(_, _, line)| home.is_match(line))
        .map(|(file, n, line)| format!("{file}:{n}: {}", line.trim()))
        .collect();
    report(
        "home-directory paths",
        "use a path relative to the repository root, or an environment variable",
        &hits,
    );
}

#[test]
fn no_email_addresses() {
    let email = Regex::new(r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").unwrap();
    let allowed = [
        Regex::new(r"^\d+\+[A-Za-z0-9-]+@users\.noreply\.github\.com$").unwrap(),
        Regex::new(r"^noreply@(?:anthropic|github)\.com$").unwrap(),
        Regex::new(r"@(?:example|invalid|localhost|test)(?:\.[A-Za-z]{2,})?$").unwrap(),
        Regex::new(r"\.(?:example|invalid|test)$").unwrap(),
    ];
    let files = text_files();
    let mut hits = Vec::new();
    for (file, n, line) in lines(&files) {
        let name = file.rsplit('/').next().unwrap_or(file);
        if EMAIL_EXEMPT_FILES.contains(&name) {
            continue;
        }
        for found in email.find_iter(&line) {
            let address = found.as_str();
            if !allowed.iter().any(|ok| ok.is_match(address)) {
                hits.push(format!("{file}:{n}: {address}"));
            }
        }
    }
    report(
        "email addresses that are not documentation or noreply addresses",
        "a real address in a public repository cannot be taken back",
        &hits,
    );
}

#[test]
fn no_secret_shaped_strings() {
    let secrets = [
        r"\bpk_(?:live|test)_[A-Za-z0-9]{16,}",
        r"\bpbt_[A-Za-z0-9]{16,}",
        r"\bsk-[A-Za-z0-9_-]{20,}",
        r"\bAIza[0-9A-Za-z_-]{35}",
        r"\bgh[pousr]_[A-Za-z0-9]{30,}",
        r"\bgithub_pat_[A-Za-z0-9_]{30,}",
        r"\bxox[baprs]-[A-Za-z0-9-]{10,}",
        r"-----BEGIN (?:RSA |EC |OPENSSH |)PRIVATE KEY-----",
        r"\bpat_[A-Za-z0-9_-]{16,}",
    ]
    .map(|pattern| Regex::new(pattern).unwrap());
    let files = text_files();
    let hits: Vec<String> = lines(&files)
        .filter(|(file, _, _)| *file != APP_TOKEN_FILE)
        .filter(|(_, _, line)| secrets.iter().any(|re| re.is_match(line)))
        .map(|(file, n, line)| format!("{file}:{n}: {}", line.trim()))
        .collect();
    report(
        "strings shaped like keys or tokens",
        "remove the key and rotate it",
        &hits,
    );
}

/// Every document module 00 promises exists and is not empty (docs/modules/00-scaffold.md,
/// Scope 11 and acceptance test 13).
#[test]
fn required_documents_exist() {
    let root = repository_root();
    let required = [
        "README.md",
        "CONTRIBUTING.md",
        "SECURITY.md",
        "RELEASING.md",
        "THIRD_PARTY.md",
        "LICENSE-MIT",
        "LICENSE-APACHE",
        "docs/HANDBOOK.md",
        "docs/PRODUCT.md",
        "docs/ARCHITECTURE.md",
        "docs/PUBLIK-INTEGRATION.md",
        "docs/PUBLIK-CONTRACT.md",
        "docs/TESTING.md",
        "docs/ROADMAP.md",
        "docs/adr/README.md",
        "docs/adr/template.md",
    ];
    let mut missing = Vec::new();
    for name in required {
        let path = root.join(name);
        if fs::metadata(&path).map(|m| m.len()).unwrap_or(0) == 0 {
            missing.push(name.to_string());
        }
    }
    let adrs: BTreeSet<String> = fs::read_dir(root.join("docs").join("adr"))
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(|e| e.metadata().map(|m| m.len() > 0).unwrap_or(false))
                .filter_map(|e| e.file_name().to_str().map(str::to_string))
                .collect()
        })
        .unwrap_or_default();
    for number in 1..=7 {
        let prefix = format!("{number:04}-");
        if !adrs
            .iter()
            .any(|name| name.starts_with(&prefix) && name.ends_with(".md"))
        {
            missing.push(format!("docs/adr/{prefix}*.md"));
        }
    }
    assert!(
        missing.is_empty(),
        "required documents missing or empty: {missing:?}"
    );
}

/// The licence gate refuses copyleft licences (docs/modules/00-scaffold.md, acceptance test 12).
/// cargo-deny refuses every licence outside `allow`, so the gate is the allow list; the refused
/// licences are named in `deny.toml` so the policy is explicit, and the npm checker's `--onlyAllow`
/// list is the same.
#[test]
fn licence_gate_refuses_copyleft() {
    let root = repository_root();
    let allowed = [
        "MIT",
        "Apache-2.0",
        "BSD-2-Clause",
        "BSD-3-Clause",
        "ISC",
        "Zlib",
        "Unicode-3.0",
        "MPL-2.0",
    ];
    let refused = ["GPL-2.0", "GPL-3.0", "AGPL-3.0", "SSPL-1.0", "BUSL-1.1"];

    let deny_text = fs::read_to_string(root.join("deny.toml")).expect("deny.toml exists");
    let deny: toml::Table = toml::from_str(&deny_text).expect("deny.toml parses");
    let allow: Vec<&str> = deny["licenses"]["allow"]
        .as_array()
        .expect("licenses.allow is a list")
        .iter()
        .filter_map(toml::Value::as_str)
        .collect();
    for licence in allowed {
        assert!(
            allow.contains(&licence),
            "deny.toml does not allow {licence}"
        );
    }
    for licence in refused {
        assert!(!allow.contains(&licence), "deny.toml allows {licence}");
        assert!(
            allow.iter().all(|a| !a.starts_with(licence)),
            "deny.toml allows a variant of {licence}"
        );
        assert!(
            deny_text.contains(licence),
            "deny.toml does not name {licence} among the refused licences"
        );
    }

    let package_text =
        fs::read_to_string(root.join("app").join("package.json")).expect("app/package.json exists");
    let package: serde_json::Value =
        serde_json::from_str(&package_text).expect("package.json parses");
    let licences_script = package["scripts"]["licenses"]
        .as_str()
        .expect("app/package.json has a licenses script");
    let expected = format!("--onlyAllow \"{}\"", allowed.join(";"));
    assert!(
        licences_script.contains(&expected),
        "the npm licence check does not use the same allow list: {licences_script}"
    );
}

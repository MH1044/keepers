//! The golden harness: runs every case under `tests/cases/NN-slug/<case>/` against the `keepers`
//! binary Cargo built, and compares the output with the rules in `docs/TESTING.md` §3.
//!
//! `GOLDEN_FILTER=<module>` selects one module's cases. `UPDATE_GOLDEN=1` rewrites the expected
//! files of the filtered cases, and is refused without a filter: a golden file is the
//! specification, not a cache, so regeneration is deliberate and narrow.

#[path = "support/compare.rs"]
mod compare;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use compare::{PointerPattern, Rules, Tolerance};
use regex::Regex;
use serde::Deserialize;
use serde_json::Value;

const DEFAULT_FIXED_CLOCK: &str = "2026-01-01T00:00:00.000Z";

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct Case {
    #[allow(dead_code)]
    description: String,
    fixture: Option<String>,
    #[serde(default)]
    copy_fixture: bool,
    args: Vec<String>,
    exit_code: i32,
    stdout: Option<String>,
    #[serde(default)]
    requires_models: Vec<String>,
    #[serde(default)]
    requires: Vec<String>,
    #[serde(default)]
    env: BTreeMap<String, String>,
    #[serde(default)]
    tolerance: BTreeMap<String, ToleranceSpec>,
    #[serde(default, rename = "match")]
    matches: BTreeMap<String, String>,
    #[serde(default)]
    ignore: IgnoreSpec,
    #[serde(default)]
    files: BTreeMap<String, String>,
}

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
struct ToleranceSpec {
    abs: Option<f64>,
    rel: Option<f64>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct IgnoreSpec {
    #[serde(default)]
    paths: Vec<String>,
}

/// Whether expected files are rewritten; refused without a module filter.
fn update_mode(update: Option<&str>, filter: Option<&str>) -> Result<bool, String> {
    match update {
        Some(value) if !value.is_empty() && value != "0" => {
            if filter.is_none_or(str::is_empty) {
                Err("UPDATE_GOLDEN needs GOLDEN_FILTER=<module>".to_string())
            } else {
                Ok(true)
            }
        }
        _ => Ok(false),
    }
}

fn repository_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("tests").join("cases").is_dir() {
        assert!(dir.pop(), "tests/cases was not found above the crate");
    }
    dir
}

fn rules_of(case: &Case) -> Result<Rules, String> {
    let mut rules = Rules::default();
    for (pointer, spec) in &case.tolerance {
        let tolerance = match (spec.abs, spec.rel) {
            (Some(abs), None) => Tolerance::Abs(abs),
            (None, Some(rel)) => Tolerance::Rel(rel),
            _ => {
                return Err(format!(
                    "tolerance {pointer} needs exactly one of abs or rel"
                ));
            }
        };
        rules
            .tolerances
            .push((PointerPattern::parse(pointer)?, tolerance));
    }
    for (pointer, pattern) in &case.matches {
        let re = Regex::new(pattern).map_err(|e| format!("match {pointer}: {e}"))?;
        rules.matches.push((PointerPattern::parse(pointer)?, re));
    }
    for pointer in &case.ignore.paths {
        rules.ignores.push(PointerPattern::parse(pointer)?);
    }
    Ok(rules)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::create_dir_all(to)?;
    for entry in fs::read_dir(from)? {
        let entry = entry?;
        let target = to.join(entry.file_name());
        if entry.file_type()?.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

fn skip_reason(case: &Case, root: &Path) -> Option<String> {
    let models_dir = std::env::var_os("KEEPERS_MODELS_DIR").map(PathBuf::from);
    for id in &case.requires_models {
        let installed = models_dir.as_ref().is_some_and(|dir| dir.join(id).is_dir());
        if !installed {
            return Some(format!(
                "skipped: model {id} not installed (scripts/fetch-models.sh)"
            ));
        }
    }
    for requirement in &case.requires {
        let available = match requirement.as_str() {
            "raw-samples" => root.join("tests").join("fixtures").join("raw").is_dir(),
            _ => false,
        };
        if !available {
            return Some(match requirement.as_str() {
                "raw-samples" => {
                    "skipped: raw samples not downloaded (scripts/fetch-raw-samples.sh)".to_string()
                }
                other => format!("skipped: {other} is not available"),
            });
        }
    }
    None
}

/// Runs one case; returns its failures (empty when it passed).
fn run_case(root: &Path, case_dir: &Path, update: bool) -> Result<Vec<String>, String> {
    let case_text = fs::read_to_string(case_dir.join("case.toml"))
        .map_err(|e| format!("cannot read case.toml: {e}"))?;
    let case: Case = toml::from_str(&case_text).map_err(|e| format!("case.toml: {e}"))?;
    if let Some(reason) = skip_reason(&case, root) {
        println!("{reason}");
        return Ok(Vec::new());
    }
    let rules = rules_of(&case)?;

    let tmp = tempfile::tempdir().map_err(|e| format!("temp dir: {e}"))?;
    let data_dir = tmp.path().join("data");
    fs::create_dir_all(&data_dir).map_err(|e| format!("data dir: {e}"))?;
    let fixture = match &case.fixture {
        Some(name) => {
            let source = root.join("tests").join("fixtures").join(name);
            if !source.is_dir() {
                return Err(format!(
                    "fixture {name} is missing; run the fixture generator (docs/TESTING.md §2)"
                ));
            }
            if case.copy_fixture {
                let copy = tmp.path().join("fixture");
                copy_dir(&source, &copy).map_err(|e| format!("copying the fixture: {e}"))?;
                Some(copy)
            } else {
                Some(source)
            }
        }
        None => None,
    };
    let substitute = |text: &str| -> Result<String, String> {
        let mut out = text.replace("{tmp}", &data_dir.to_string_lossy());
        if out.contains("{fixture}") {
            let fixture = fixture
                .as_ref()
                .ok_or_else(|| "{fixture} used but the case names no fixture".to_string())?;
            out = out.replace("{fixture}", &fixture.to_string_lossy());
        }
        Ok(out)
    };
    let args = case
        .args
        .iter()
        .map(|a| substitute(a))
        .collect::<Result<Vec<_>, _>>()?;

    let mut command = Command::new(env!("CARGO_BIN_EXE_keepers"));
    command
        .args(&args)
        .current_dir(root)
        .env("KEEPERS_FIXED_CLOCK", DEFAULT_FIXED_CLOCK)
        .env("KEEPERS_DATA_DIR", &data_dir);
    for (key, value) in &case.env {
        command.env(key, substitute(value)?);
    }
    let output = command
        .output()
        .map_err(|e| format!("running keepers: {e}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();

    let mut failures = Vec::new();
    let exit = output.status.code().unwrap_or(-1);
    if exit != case.exit_code {
        failures.push(format!(
            "exit code: expected {}, got {exit}\n    stdout: {}\n    stderr: {}",
            case.exit_code,
            stdout.trim(),
            stderr.trim()
        ));
    }

    if let Some(expected_name) = &case.stdout {
        let expected_path = case_dir.join(expected_name);
        let actual: Value = match serde_json::from_str(&stdout) {
            Ok(value) => value,
            Err(e) => {
                failures.push(format!("stdout is not JSON ({e}): {}", stdout.trim()));
                Value::Null
            }
        };
        if update {
            let mut pretty = serde_json::to_string_pretty(&actual).expect("JSON serialises");
            pretty.push('\n');
            fs::write(&expected_path, pretty).map_err(|e| format!("writing expected: {e}"))?;
        } else if actual != Value::Null {
            let expected_text = fs::read_to_string(&expected_path)
                .map_err(|e| format!("cannot read {expected_name}: {e}"))?;
            let expected: Value = serde_json::from_str(&expected_text)
                .map_err(|e| format!("{expected_name} is not JSON: {e}"))?;
            failures.extend(compare::compare_json(&expected, &actual, &rules));
        }
    }

    for (written, expected_rel) in &case.files {
        let written_path = PathBuf::from(substitute(written)?);
        let expected_path = case_dir.join(expected_rel);
        let actual_bytes = match fs::read(&written_path) {
            Ok(bytes) => bytes,
            Err(e) => {
                failures.push(format!("{written}: not written ({e})"));
                continue;
            }
        };
        if update {
            if let Some(parent) = expected_path.parent() {
                fs::create_dir_all(parent).map_err(|e| format!("creating {expected_rel}: {e}"))?;
            }
            fs::write(&expected_path, &actual_bytes)
                .map_err(|e| format!("writing {expected_rel}: {e}"))?;
            continue;
        }
        let expected_bytes =
            fs::read(&expected_path).map_err(|e| format!("cannot read {expected_rel}: {e}"))?;
        if let Some(diff) = compare::compare_bytes(expected_rel, &expected_bytes, &actual_bytes) {
            failures.push(diff);
        }
    }
    Ok(failures)
}

#[test]
fn golden_cases() {
    let update_env = std::env::var("UPDATE_GOLDEN").ok();
    let filter = std::env::var("GOLDEN_FILTER").ok();
    let update = update_mode(update_env.as_deref(), filter.as_deref())
        .unwrap_or_else(|message| panic!("{message}"));

    let root = repository_root();
    let cases_root = root.join("tests").join("cases");
    let mut modules: Vec<PathBuf> = fs::read_dir(&cases_root)
        .expect("tests/cases is readable")
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    modules.sort();

    let mut ran = 0;
    let mut report = Vec::new();
    for module_dir in modules {
        let module = module_dir
            .file_name()
            .unwrap()
            .to_string_lossy()
            .into_owned();
        if let Some(f) = filter.as_deref().filter(|f| !f.is_empty())
            && module != f
            && !module.starts_with(&format!("{f}-"))
        {
            continue;
        }
        let mut cases: Vec<PathBuf> = fs::read_dir(&module_dir)
            .expect("module directory is readable")
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| p.join("case.toml").is_file())
            .collect();
        cases.sort();
        for case_dir in cases {
            let name = format!(
                "{module}/{}",
                case_dir.file_name().unwrap().to_string_lossy()
            );
            ran += 1;
            match run_case(&root, &case_dir, update) {
                Ok(failures) if failures.is_empty() => println!("golden {name}: ok"),
                Ok(failures) => {
                    println!("golden {name}: FAILED");
                    report.push(format!("{name}:\n  {}", failures.join("\n  ")));
                }
                Err(error) => {
                    println!("golden {name}: ERROR");
                    report.push(format!("{name}: {error}"));
                }
            }
        }
    }
    assert!(
        ran > 0,
        "no golden cases matched (GOLDEN_FILTER={filter:?})"
    );
    assert!(
        report.is_empty(),
        "{} golden case(s) failed:\n{}",
        report.len(),
        report.join("\n")
    );
}

#[test]
fn update_golden_needs_a_filter() {
    assert_eq!(
        update_mode(Some("1"), None),
        Err("UPDATE_GOLDEN needs GOLDEN_FILTER=<module>".to_string())
    );
    assert_eq!(
        update_mode(Some("1"), Some("")),
        Err("UPDATE_GOLDEN needs GOLDEN_FILTER=<module>".to_string())
    );
    assert_eq!(update_mode(Some("1"), Some("00-scaffold")), Ok(true));
    assert_eq!(update_mode(Some("0"), None), Ok(false));
    assert_eq!(update_mode(None, None), Ok(false));
    assert_eq!(update_mode(None, Some("00-scaffold")), Ok(false));
}

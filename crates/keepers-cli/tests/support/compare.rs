//! The comparison rules of the golden harness (`docs/TESTING.md` §3), with their own tests.
//!
//! A JSON Pointer names every difference. Keys missing from either side fail; numbers without a
//! tolerance compare exactly; `*` in a rule's pointer matches any array index.

use std::collections::BTreeSet;
use std::fmt;

use regex::Regex;
use serde_json::Value;

/// How far a number may be from its expected value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Tolerance {
    /// `|actual - expected| <= abs`
    Abs(f64),
    /// `|actual - expected| <= rel * |expected|`
    Rel(f64),
}

/// A JSON Pointer in which `*` stands for any array index.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PointerPattern {
    segments: Vec<String>,
}

impl PointerPattern {
    /// Parses `/a/0/b` or `/frames/*/sharpness`; the empty string is the whole document.
    pub fn parse(pointer: &str) -> Result<Self, String> {
        if pointer.is_empty() {
            return Ok(PointerPattern {
                segments: Vec::new(),
            });
        }
        let rest = pointer
            .strip_prefix('/')
            .ok_or_else(|| format!("pointer '{pointer}' must start with '/'"))?;
        Ok(PointerPattern {
            segments: rest
                .split('/')
                .map(|s| s.replace("~1", "/").replace("~0", "~"))
                .collect(),
        })
    }

    fn matches(&self, path: &[String]) -> bool {
        self.segments.len() == path.len()
            && self
                .segments
                .iter()
                .zip(path)
                .all(|(pattern, actual)| pattern == "*" || pattern == actual)
    }
}

/// The rules of one case.
#[derive(Default)]
pub struct Rules {
    pub tolerances: Vec<(PointerPattern, Tolerance)>,
    pub matches: Vec<(PointerPattern, Regex)>,
    pub ignores: Vec<PointerPattern>,
}

impl Rules {
    fn tolerance_for(&self, path: &[String]) -> Option<Tolerance> {
        self.tolerances
            .iter()
            .find(|(pattern, _)| pattern.matches(path))
            .map(|(_, t)| *t)
    }

    fn match_for(&self, path: &[String]) -> Option<&Regex> {
        self.matches
            .iter()
            .find(|(pattern, _)| pattern.matches(path))
            .map(|(_, re)| re)
    }

    fn is_ignored(&self, path: &[String]) -> bool {
        self.ignores.iter().any(|pattern| pattern.matches(path))
    }
}

struct Pointer<'a>(&'a [String]);

impl fmt::Display for Pointer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.is_empty() {
            return write!(f, "(document)");
        }
        for segment in self.0 {
            write!(f, "/{}", segment.replace('~', "~0").replace('/', "~1"))?;
        }
        Ok(())
    }
}

/// Compares `actual` against `expected` and returns every difference, each naming its pointer.
pub fn compare_json(expected: &Value, actual: &Value, rules: &Rules) -> Vec<String> {
    let mut failures = Vec::new();
    walk(expected, actual, &mut Vec::new(), rules, &mut failures);
    failures
}

fn walk(
    expected: &Value,
    actual: &Value,
    path: &mut Vec<String>,
    rules: &Rules,
    failures: &mut Vec<String>,
) {
    if rules.is_ignored(path) {
        return;
    }
    if let Some(re) = rules.match_for(path) {
        match actual {
            Value::String(s) if re.is_match(s) => {}
            Value::String(s) => failures.push(format!(
                "{}: {s:?} does not match the pattern {}",
                Pointer(path),
                re.as_str()
            )),
            other => failures.push(format!(
                "{}: expected a string matching {}, got {other}",
                Pointer(path),
                re.as_str()
            )),
        }
        return;
    }
    match (expected, actual) {
        (Value::Object(exp), Value::Object(act)) => {
            let keys: BTreeSet<&String> = exp.keys().chain(act.keys()).collect();
            for key in keys {
                path.push(key.clone());
                if !rules.is_ignored(path) {
                    match (exp.get(key), act.get(key)) {
                        (Some(e), Some(a)) => walk(e, a, path, rules, failures),
                        (Some(_), None) => failures
                            .push(format!("{}: missing from the actual output", Pointer(path))),
                        (None, Some(_)) => failures.push(format!(
                            "{}: unexpected key in the actual output",
                            Pointer(path)
                        )),
                        (None, None) => unreachable!("key came from one of the two objects"),
                    }
                }
                path.pop();
            }
        }
        (Value::Array(exp), Value::Array(act)) => {
            if exp.len() != act.len() {
                failures.push(format!(
                    "{}: expected {} elements, got {}",
                    Pointer(path),
                    exp.len(),
                    act.len()
                ));
            }
            for (i, (e, a)) in exp.iter().zip(act).enumerate() {
                path.push(i.to_string());
                walk(e, a, path, rules, failures);
                path.pop();
            }
        }
        (Value::Number(exp), Value::Number(act)) => {
            let e = exp.as_f64().unwrap_or(f64::NAN);
            let a = act.as_f64().unwrap_or(f64::NAN);
            match rules.tolerance_for(path) {
                Some(Tolerance::Abs(abs)) => {
                    // Written so that a NaN on either side fails, as it should.
                    let within = (a - e).abs() <= abs;
                    if !within {
                        failures.push(format!(
                            "{}: expected {exp}, got {act} (abs tolerance {abs})",
                            Pointer(path)
                        ));
                    }
                }
                Some(Tolerance::Rel(rel)) => {
                    let within = (a - e).abs() <= rel * e.abs();
                    if !within {
                        failures.push(format!(
                            "{}: expected {exp}, got {act} (rel tolerance {rel})",
                            Pointer(path)
                        ));
                    }
                }
                None => {
                    let same = match (exp.as_i64(), act.as_i64(), exp.as_u64(), act.as_u64()) {
                        (Some(x), Some(y), _, _) => x == y,
                        (_, _, Some(x), Some(y)) => x == y,
                        _ => a == e,
                    };
                    if !same {
                        failures.push(format!("{}: expected {exp}, got {act}", Pointer(path)));
                    }
                }
            }
        }
        _ if expected == actual => {}
        _ => failures.push(format!(
            "{}: expected {expected}, got {actual}",
            Pointer(path)
        )),
    }
}

/// Compares two files byte for byte; names the file and the first differing offset.
pub fn compare_bytes(label: &str, expected: &[u8], actual: &[u8]) -> Option<String> {
    if let Some(offset) = expected.iter().zip(actual).position(|(e, a)| e != a) {
        return Some(format!(
            "{label}: differs at byte offset {offset} (expected 0x{:02X}, got 0x{:02X})",
            expected[offset], actual[offset]
        ));
    }
    if expected.len() != actual.len() {
        return Some(format!(
            "{label}: differs at byte offset {} (expected {} bytes, got {})",
            expected.len().min(actual.len()),
            expected.len(),
            actual.len()
        ));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn rules_with_abs(pointer: &str, abs: f64) -> Rules {
        Rules {
            tolerances: vec![(PointerPattern::parse(pointer).unwrap(), Tolerance::Abs(abs))],
            ..Rules::default()
        }
    }

    #[test]
    fn within_abs_tolerance_passes() {
        let failures = compare_json(
            &json!({"v": 1.0}),
            &json!({"v": 1.0005}),
            &rules_with_abs("/v", 1e-3),
        );
        assert!(failures.is_empty(), "{failures:?}");
    }

    #[test]
    fn outside_abs_tolerance_fails() {
        let failures = compare_json(
            &json!({"v": 1.0}),
            &json!({"v": 1.002}),
            &rules_with_abs("/v", 1e-3),
        );
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].starts_with("/v: expected 1.0, got 1.002"),
            "{}",
            failures[0]
        );
    }

    #[test]
    fn rel_tolerance_scales_with_the_expected_value() {
        let rules = Rules {
            tolerances: vec![(
                PointerPattern::parse("/frames/*/lap_var").unwrap(),
                Tolerance::Rel(1e-4),
            )],
            ..Rules::default()
        };
        let expected = json!({"frames": [{"lap_var": 10000.0}, {"lap_var": 10000.0}]});
        let within = json!({"frames": [{"lap_var": 10000.5}, {"lap_var": 9999.5}]});
        assert!(compare_json(&expected, &within, &rules).is_empty());
        let outside = json!({"frames": [{"lap_var": 10000.0}, {"lap_var": 10002.0}]});
        let failures = compare_json(&expected, &outside, &rules);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].starts_with("/frames/1/lap_var:"),
            "{}",
            failures[0]
        );
    }

    #[test]
    fn extra_key_fails() {
        let failures = compare_json(
            &json!({"a": 1}),
            &json!({"a": 1, "extra": 2}),
            &Rules::default(),
        );
        assert_eq!(
            failures,
            vec!["/extra: unexpected key in the actual output"]
        );
    }

    #[test]
    fn missing_key_fails() {
        let failures = compare_json(
            &json!({"a": 1, "b": 2}),
            &json!({"a": 1}),
            &Rules::default(),
        );
        assert_eq!(failures, vec!["/b: missing from the actual output"]);
    }

    #[test]
    fn ignored_path_passes_whatever_its_value() {
        let rules = Rules {
            ignores: vec![PointerPattern::parse("/summary/elapsed_ms").unwrap()],
            ..Rules::default()
        };
        let expected = json!({"summary": {"elapsed_ms": 12, "frames": 3}});
        for actual in [
            json!({"summary": {"elapsed_ms": 9999, "frames": 3}}),
            json!({"summary": {"elapsed_ms": "slow", "frames": 3}}),
            json!({"summary": {"frames": 3}}),
        ] {
            let failures = compare_json(&expected, &actual, &rules);
            assert!(failures.is_empty(), "{actual}: {failures:?}");
        }
        let changed = json!({"summary": {"elapsed_ms": 12, "frames": 4}});
        assert_eq!(compare_json(&expected, &changed, &rules).len(), 1);
    }

    #[test]
    fn match_failure_names_the_pointer() {
        let rules = Rules {
            matches: vec![(
                PointerPattern::parse("/version").unwrap(),
                Regex::new(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$").unwrap(),
            )],
            ..Rules::default()
        };
        let expected = json!({"version": "0.1.0"});
        assert!(compare_json(&expected, &json!({"version": "2.3.4-rc.1"}), &rules).is_empty());
        let failures = compare_json(&expected, &json!({"version": "latest"}), &rules);
        assert_eq!(failures.len(), 1);
        assert!(
            failures[0].starts_with("/version: \"latest\" does not match"),
            "{}",
            failures[0]
        );
        let failures = compare_json(&expected, &json!({"version": 1}), &rules);
        assert!(
            failures[0].starts_with("/version: expected a string"),
            "{}",
            failures[0]
        );
    }

    #[test]
    fn exact_numbers_without_tolerance() {
        assert!(compare_json(&json!({"n": 3}), &json!({"n": 3}), &Rules::default()).is_empty());
        assert!(compare_json(&json!({"n": 1.5}), &json!({"n": 1.5}), &Rules::default()).is_empty());
        let failures = compare_json(&json!({"n": 3}), &json!({"n": 3.0001}), &Rules::default());
        assert_eq!(failures, vec!["/n: expected 3, got 3.0001"]);
    }

    #[test]
    fn arrays_report_length_and_elements() {
        let failures = compare_json(&json!([1, 2, 3]), &json!([1, 2]), &Rules::default());
        assert_eq!(failures, vec!["(document): expected 3 elements, got 2"]);
        let failures = compare_json(
            &json!({"a": [1, 2]}),
            &json!({"a": [1, 5]}),
            &Rules::default(),
        );
        assert_eq!(failures, vec!["/a/1: expected 2, got 5"]);
    }

    #[test]
    fn pointer_escapes_round_trip() {
        let pattern = PointerPattern::parse("/a~1b/c~0d").unwrap();
        assert!(pattern.matches(&["a/b".to_string(), "c~d".to_string()]));
        assert_eq!(
            Pointer(&["a/b".to_string(), "c~d".to_string()]).to_string(),
            "/a~1b/c~0d"
        );
        assert!(PointerPattern::parse("no-slash").is_err());
    }

    #[test]
    fn byte_difference_names_the_file_and_offset() {
        assert_eq!(compare_bytes("IMG_0001.xmp", b"abc", b"abc"), None);
        assert_eq!(
            compare_bytes("IMG_0001.xmp", b"abcdef", b"abcxef"),
            Some("IMG_0001.xmp: differs at byte offset 3 (expected 0x64, got 0x78)".to_string())
        );
        assert_eq!(
            compare_bytes("IMG_0001.xmp", b"abc", b"abcd"),
            Some("IMG_0001.xmp: differs at byte offset 3 (expected 3 bytes, got 4)".to_string())
        );
    }
}

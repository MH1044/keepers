//! `scripts/release-check.sh` refuses a release without its notes
//! (docs/modules/00-scaffold.md, acceptance test 11).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn repository_root() -> PathBuf {
    let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    while !dir.join("scripts").join("release-check.sh").is_file() {
        assert!(
            dir.pop(),
            "scripts/release-check.sh was not found above the crate"
        );
    }
    dir
}

/// A POSIX shell, if the machine has one (Git for Windows ships `sh` and `bash`).
fn shell() -> Option<&'static str> {
    ["sh", "bash"]
        .into_iter()
        .find(|name| Command::new(name).arg("-c").arg("exit 0").status().is_ok())
}

fn run(shell: &str, script: &Path, version: &str) -> (i32, String) {
    let output = Command::new(shell)
        .arg(script)
        .arg(version)
        .output()
        .expect("the shell runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn release_check_refuses_a_version_without_notes_and_accepts_one_with_them() {
    let Some(shell) = shell() else {
        println!("skipped: no POSIX shell found to run scripts/release-check.sh");
        return;
    };
    // Copy the script into a temporary tree so the test can plant notes without touching docs/.
    let tmp = tempfile::tempdir().unwrap();
    let scripts = tmp.path().join("scripts");
    fs::create_dir_all(&scripts).unwrap();
    let script = scripts.join("release-check.sh");
    fs::copy(
        repository_root().join("scripts").join("release-check.sh"),
        &script,
    )
    .unwrap();

    let (code, stderr) = run(shell, &script, "0.1.0");
    assert_eq!(code, 1, "stderr: {stderr}");
    assert!(
        stderr.contains("docs/releases/0.1.0.md is missing; write the release notes first."),
        "stderr: {stderr}"
    );

    let releases = tmp.path().join("docs").join("releases");
    fs::create_dir_all(&releases).unwrap();
    fs::write(
        releases.join("0.1.0.md"),
        "# Keepers 0.1.0\n\nFirst release.\n",
    )
    .unwrap();
    let (code, stderr) = run(shell, &script, "0.1.0");
    assert_eq!(code, 0, "stderr: {stderr}");

    let (code, stderr) = run(shell, &script, "");
    assert_eq!(code, 2, "stderr: {stderr}");
}

//! `keepers`: the command-line front end of the Keepers core.
//!
//! Every document printed with `--json` starts with a `schema` field. Errors print the
//! `keepers.error/1` document and exit with the codes in `docs/ARCHITECTURE.md` §5.

use std::ffi::OsString;
use std::io::Write;

use clap::error::ErrorKind;
use clap::{Parser, Subcommand};
use keepers_core::env::{Clock, FixedClock, SystemClock, Timestamp};
use keepers_core::error::{ErrorDocument, ExitCode};
use serde::Serialize;

const VERSION_SCHEMA: &str = "keepers.version/1";
const HELP_HINT: &str = "Run keepers --help.";

#[derive(Parser, Debug)]
#[command(
    name = "keepers",
    about = "Culls a photo shoot on your own computer and explains every decision.",
    disable_version_flag = true,
    disable_help_subcommand = true
)]
struct Cli {
    /// Print the version
    #[arg(long)]
    version: bool,

    /// Print documents as JSON
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Any command this version does not know.
    #[command(external_subcommand)]
    Unknown(Vec<OsString>),
}

#[derive(Serialize)]
struct VersionDocument {
    schema: &'static str,
    name: &'static str,
    version: &'static str,
}

/// What a run produced, separated so tests can check it without a process.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    exit: ExitCode,
    stdout: String,
    stderr: String,
}

impl Outcome {
    fn ok(stdout: String) -> Self {
        Outcome {
            exit: ExitCode::Ok,
            stdout,
            stderr: String::new(),
        }
    }

    fn error(exit: ExitCode, doc: &ErrorDocument, json: bool) -> Self {
        if json {
            let mut line = serde_json::to_string(doc).expect("an error document serialises");
            line.push('\n');
            Outcome {
                exit,
                stdout: line,
                stderr: String::new(),
            }
        } else {
            Outcome {
                exit,
                stdout: String::new(),
                stderr: format!("error: {}\n", doc.message),
            }
        }
    }
}

/// `KEEPERS_FIXED_CLOCK=<rfc3339>` selects a fixed clock (tests only); otherwise the system clock.
fn clock_from_env(value: Option<&str>) -> Result<Box<dyn Clock>, ErrorDocument> {
    match value {
        None | Some("") => Ok(Box::new(SystemClock)),
        Some(text) => {
            let instant: Timestamp = text.parse().map_err(|_| {
                ErrorDocument::usage(format!(
                    "KEEPERS_FIXED_CLOCK is not an RFC 3339 timestamp: '{text}'."
                ))
            })?;
            Ok(Box::new(FixedClock(instant)))
        }
    }
}

fn wants_json(argv: &[OsString]) -> bool {
    argv.iter().skip(1).any(|arg| arg == "--json")
}

fn run(argv: &[OsString], fixed_clock: Option<&str>) -> Outcome {
    let json = wants_json(argv);
    // Later commands take the clock; parsing it here makes a bad value a usage error everywhere.
    let _clock = match clock_from_env(fixed_clock) {
        Ok(clock) => clock,
        Err(doc) => return Outcome::error(ExitCode::Usage, &doc, json),
    };

    let cli = match Cli::try_parse_from(argv) {
        Ok(cli) => cli,
        Err(err) if err.kind() == ErrorKind::DisplayHelp => {
            return Outcome::ok(err.to_string());
        }
        Err(err) => {
            let first_line = err
                .to_string()
                .lines()
                .next()
                .unwrap_or("invalid arguments")
                .trim_start_matches("error: ")
                .to_string();
            let doc = ErrorDocument::usage(format!("{first_line}. {HELP_HINT}"));
            return Outcome::error(ExitCode::Usage, &doc, json);
        }
    };

    if cli.version {
        let doc = VersionDocument {
            schema: VERSION_SCHEMA,
            name: "keepers",
            version: env!("CARGO_PKG_VERSION"),
        };
        let text = if json {
            format!(
                "{}\n",
                serde_json::to_string(&doc).expect("a version document serialises")
            )
        } else {
            format!("keepers {}\n", doc.version)
        };
        return Outcome::ok(text);
    }

    match cli.command {
        Some(Command::Unknown(args)) => {
            let name = args
                .first()
                .map(|a| a.to_string_lossy().into_owned())
                .unwrap_or_default();
            let doc = ErrorDocument::usage(format!("unknown command '{name}'. {HELP_HINT}"));
            Outcome::error(ExitCode::Usage, &doc, json)
        }
        None => {
            let doc = ErrorDocument::usage(format!("no command given. {HELP_HINT}"));
            Outcome::error(ExitCode::Usage, &doc, json)
        }
    }
}

fn main() {
    let argv: Vec<OsString> = std::env::args_os().collect();
    let fixed_clock = std::env::var("KEEPERS_FIXED_CLOCK").ok();
    let outcome = run(&argv, fixed_clock.as_deref());
    let mut stdout = std::io::stdout().lock();
    let mut stderr = std::io::stderr().lock();
    let _ = stdout.write_all(outcome.stdout.as_bytes());
    let _ = stderr.write_all(outcome.stderr.as_bytes());
    let _ = stdout.flush();
    let _ = stderr.flush();
    std::process::exit(outcome.exit.code());
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(args: &[&str]) -> Vec<OsString> {
        std::iter::once("keepers")
            .chain(args.iter().copied())
            .map(OsString::from)
            .collect()
    }

    #[test]
    fn version_as_json_is_the_version_document() {
        let out = run(&argv(&["--version", "--json"]), None);
        assert_eq!(out.exit, ExitCode::Ok);
        assert_eq!(
            out.stdout,
            format!(
                "{{\"schema\":\"keepers.version/1\",\"name\":\"keepers\",\"version\":\"{}\"}}\n",
                env!("CARGO_PKG_VERSION")
            )
        );
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn version_as_text_is_one_line() {
        let out = run(&argv(&["--version"]), None);
        assert_eq!(out.exit, ExitCode::Ok);
        assert_eq!(
            out.stdout,
            format!("keepers {}\n", env!("CARGO_PKG_VERSION"))
        );
    }

    #[test]
    fn unknown_command_is_the_usage_document() {
        let out = run(&argv(&["frobnicate", "--json"]), None);
        assert_eq!(out.exit, ExitCode::Usage);
        assert_eq!(
            out.stdout,
            "{\"schema\":\"keepers.error/1\",\"code\":\"usage\",\"message\":\"unknown command 'frobnicate'. Run keepers --help.\"}\n"
        );
        assert!(out.stderr.is_empty());
    }

    #[test]
    fn unknown_command_without_json_goes_to_stderr() {
        let out = run(&argv(&["frobnicate"]), None);
        assert_eq!(out.exit, ExitCode::Usage);
        assert!(out.stdout.is_empty());
        assert_eq!(
            out.stderr,
            "error: unknown command 'frobnicate'. Run keepers --help.\n"
        );
    }

    #[test]
    fn no_command_is_a_usage_error() {
        let out = run(&argv(&["--json"]), None);
        assert_eq!(out.exit, ExitCode::Usage);
        assert!(out.stdout.contains("\"code\":\"usage\""));
        assert!(out.stdout.contains("no command given. Run keepers --help."));
    }

    #[test]
    fn unexpected_flag_is_a_usage_error() {
        let out = run(&argv(&["--bogus", "--json"]), None);
        assert_eq!(out.exit, ExitCode::Usage);
        let doc: ErrorDocument = serde_json::from_str(out.stdout.trim()).unwrap();
        assert_eq!(doc.code, "usage");
        assert!(doc.message.contains("--bogus"), "{}", doc.message);
        assert!(
            doc.message.ends_with("Run keepers --help."),
            "{}",
            doc.message
        );
    }

    #[test]
    fn help_exits_zero() {
        let out = run(&argv(&["--help"]), None);
        assert_eq!(out.exit, ExitCode::Ok);
        assert!(out.stdout.contains("Usage: keepers"));
        assert!(out.stdout.contains("--json"));
    }

    #[test]
    fn fixed_clock_is_selected_by_the_environment() {
        let clock = clock_from_env(Some("2026-01-01T00:00:00.000Z")).unwrap();
        assert_eq!(clock.now_utc().to_string(), "2026-01-01T00:00:00.000Z");
        assert!(clock_from_env(None).is_ok());
        assert!(clock_from_env(Some("")).is_ok());
        let Err(err) = clock_from_env(Some("noon")) else {
            panic!("'noon' should not parse as a timestamp");
        };
        assert_eq!(err.code, "usage");
        assert_eq!(
            err.message,
            "KEEPERS_FIXED_CLOCK is not an RFC 3339 timestamp: 'noon'."
        );
    }

    #[test]
    fn a_bad_fixed_clock_fails_every_command() {
        let out = run(&argv(&["--version", "--json"]), Some("noon"));
        assert_eq!(out.exit, ExitCode::Usage);
        assert!(out.stdout.contains("KEEPERS_FIXED_CLOCK"));
    }
}

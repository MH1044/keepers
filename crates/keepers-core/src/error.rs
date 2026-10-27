//! The error document every command prints, and the process exit codes.

use serde::{Deserialize, Serialize};

/// The schema name of the error document.
pub const ERROR_SCHEMA: &str = "keepers.error/1";

/// The JSON document printed when a command fails:
/// `{"schema":"keepers.error/1","code":"usage","message":"..."}`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ErrorDocument {
    /// Always [`ERROR_SCHEMA`].
    pub schema: String,
    /// A stable, machine-readable code (`usage`, `cancelled`, ...).
    pub code: String,
    /// One sentence for a person, ending with what to do next where there is something to do.
    pub message: String,
}

impl ErrorDocument {
    /// An error document with the current schema.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        ErrorDocument {
            schema: ERROR_SCHEMA.to_string(),
            code: code.into(),
            message: message.into(),
        }
    }

    /// A usage error: the command line could not be understood.
    pub fn usage(message: impl Into<String>) -> Self {
        Self::new("usage", message)
    }
}

/// Process exit codes of the `keepers` CLI.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitCode {
    /// Everything worked.
    Ok,
    /// The command line could not be understood.
    Usage,
    /// Some files could not be read and `--strict` was given.
    PerFileErrors,
    /// A provider or model needs to be set up first.
    SetupNeeded,
    /// The chosen provider needs credit.
    NeedsCredit,
    /// The run was cancelled.
    Cancelled,
    /// An export conflict, or a plan that is no longer current.
    ExportConflict,
    /// The AI provider answered with an error.
    Ai,
    /// A bug: something that should not happen did.
    Internal,
}

impl ExitCode {
    /// The numeric code handed to the operating system.
    pub const fn code(self) -> i32 {
        match self {
            ExitCode::Ok => 0,
            ExitCode::Usage => 1,
            ExitCode::PerFileErrors => 2,
            ExitCode::SetupNeeded => 3,
            ExitCode::NeedsCredit => 4,
            ExitCode::Cancelled => 5,
            ExitCode::ExportConflict => 6,
            ExitCode::Ai => 7,
            ExitCode::Internal => 10,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_document_serialises_in_schema_order() {
        let doc = ErrorDocument::usage("unknown command 'frobnicate'. Run keepers --help.");
        assert_eq!(
            serde_json::to_string(&doc).unwrap(),
            r#"{"schema":"keepers.error/1","code":"usage","message":"unknown command 'frobnicate'. Run keepers --help."}"#
        );
        let back: ErrorDocument =
            serde_json::from_str(&serde_json::to_string(&doc).unwrap()).unwrap();
        assert_eq!(back, doc);
    }

    #[test]
    fn exit_codes_follow_the_architecture_table() {
        assert_eq!(ExitCode::Ok.code(), 0);
        assert_eq!(ExitCode::Usage.code(), 1);
        assert_eq!(ExitCode::PerFileErrors.code(), 2);
        assert_eq!(ExitCode::SetupNeeded.code(), 3);
        assert_eq!(ExitCode::NeedsCredit.code(), 4);
        assert_eq!(ExitCode::Cancelled.code(), 5);
        assert_eq!(ExitCode::ExportConflict.code(), 6);
        assert_eq!(ExitCode::Ai.code(), 7);
        assert_eq!(ExitCode::Internal.code(), 10);
    }
}

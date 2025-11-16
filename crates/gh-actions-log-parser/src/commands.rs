//! GitHub Actions workflow command parsing
//!
//! Parses workflow commands like ::group::, ::error::, ::warning::, etc.

use crate::types::{CommandParams, WorkflowCommand};
use regex::Regex;
use std::sync::OnceLock;

/// Parse a line for GitHub Actions workflow commands
///
/// Returns `Some((command, cleaned_line))` if a command is found, where `cleaned_line`
/// is the line with the command syntax removed. Returns `None` if no command is present.
pub fn parse_command(line: &str) -> Option<(WorkflowCommand, String)> {
    static COMMAND_REGEX: OnceLock<Regex> = OnceLock::new();

    let re = COMMAND_REGEX.get_or_init(|| {
        // Match ::command params::message or ::command::message
        Regex::new(r"^::([a-zA-Z-]+)(?:\s+([^:]+?))?::(.*)$").unwrap()
    });

    let captures = re.captures(line.trim())?;
    let command_name = captures.get(1)?.as_str();
    let params_str = captures.get(2).map(|m| m.as_str());
    let message = captures.get(3)?.as_str().to_string();

    let command = match command_name.to_lowercase().as_str() {
        "group" => WorkflowCommand::GroupStart {
            title: message.clone(),
        },
        "endgroup" => WorkflowCommand::GroupEnd,
        "error" => {
            let params = parse_params(params_str.unwrap_or(""));
            WorkflowCommand::Error { message: message.clone(), params }
        }
        "warning" => {
            let params = parse_params(params_str.unwrap_or(""));
            WorkflowCommand::Warning { message: message.clone(), params }
        }
        "debug" => WorkflowCommand::Debug {
            message: message.clone(),
        },
        "notice" => {
            let params = parse_params(params_str.unwrap_or(""));
            WorkflowCommand::Notice { message: message.clone(), params }
        }
        _ => return None, // Unknown command
    };

    // Return command and the message part (cleaned of command syntax)
    Some((command, message))
}

/// Parse command parameters like "file=foo.rs,line=42,col=10"
fn parse_params(params_str: &str) -> CommandParams {
    let mut params = CommandParams::default();

    for param in params_str.split(',') {
        let param = param.trim();
        if param.is_empty() {
            continue;
        }

        if let Some((key, value)) = param.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            match key {
                "file" => params.file = Some(value.to_string()),
                "line" => params.line = value.parse().ok(),
                "col" => params.col = value.parse().ok(),
                "endColumn" => params.end_column = value.parse().ok(),
                "endLine" => params.end_line = value.parse().ok(),
                "title" => params.title = Some(value.to_string()),
                _ => {} // Ignore unknown parameters
            }
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_group_start() {
        let result = parse_command("::group::Build artifacts");
        assert!(result.is_some());
        let (cmd, msg) = result.unwrap();
        assert!(matches!(cmd, WorkflowCommand::GroupStart { .. }));
        if let WorkflowCommand::GroupStart { title } = cmd {
            assert_eq!(title, "Build artifacts");
        }
        assert_eq!(msg, "Build artifacts");
    }

    #[test]
    fn test_group_end() {
        let result = parse_command("::endgroup::");
        assert!(result.is_some());
        let (cmd, msg) = result.unwrap();
        assert!(matches!(cmd, WorkflowCommand::GroupEnd));
        assert_eq!(msg, "");
    }

    #[test]
    fn test_error_with_params() {
        let result = parse_command("::error file=app.js,line=10,col=15::Something went wrong");
        assert!(result.is_some());
        let (cmd, msg) = result.unwrap();

        if let WorkflowCommand::Error { message, params } = cmd {
            assert_eq!(message, "Something went wrong");
            assert_eq!(params.file, Some("app.js".to_string()));
            assert_eq!(params.line, Some(10));
            assert_eq!(params.col, Some(15));
        } else {
            panic!("Expected Error command");
        }
        assert_eq!(msg, "Something went wrong");
    }

    #[test]
    fn test_warning_simple() {
        let result = parse_command("::warning::This is a warning");
        assert!(result.is_some());
        let (cmd, msg) = result.unwrap();

        if let WorkflowCommand::Warning { message, .. } = cmd {
            assert_eq!(message, "This is a warning");
        } else {
            panic!("Expected Warning command");
        }
        assert_eq!(msg, "This is a warning");
    }

    #[test]
    fn test_debug() {
        let result = parse_command("::debug::Debug information");
        assert!(result.is_some());
        let (cmd, msg) = result.unwrap();

        if let WorkflowCommand::Debug { message } = cmd {
            assert_eq!(message, "Debug information");
        } else {
            panic!("Expected Debug command");
        }
        assert_eq!(msg, "Debug information");
    }

    #[test]
    fn test_not_a_command() {
        let result = parse_command("This is just regular log output");
        assert!(result.is_none());
    }

    #[test]
    fn test_malformed_command() {
        let result = parse_command("::incomplete");
        assert!(result.is_none());
    }
}

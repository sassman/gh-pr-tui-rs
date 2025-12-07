//! Confirmation Popup State
//!
//! State for a reusable confirmation popup with text input.
//! Used for PR actions that require user confirmation and optional message editing.

/// The intent of the confirmation - determines what action to execute on confirm
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfirmationIntent {
    /// Approve the specified PRs with a message
    Approve { pr_numbers: Vec<u64> },
    /// Post a comment on the specified PRs
    Comment { pr_numbers: Vec<u64> },
    /// Request changes on the specified PRs
    RequestChanges { pr_numbers: Vec<u64> },
    /// Close the specified PRs with a message
    Close { pr_numbers: Vec<u64> },
}

impl ConfirmationIntent {
    /// Get the PR numbers for this intent
    pub fn pr_numbers(&self) -> &[u64] {
        match self {
            Self::Approve { pr_numbers }
            | Self::Comment { pr_numbers }
            | Self::RequestChanges { pr_numbers }
            | Self::Close { pr_numbers } => pr_numbers,
        }
    }

    /// Get a human-readable action verb for this intent
    pub fn action_verb(&self) -> &'static str {
        match self {
            Self::Approve { .. } => "Approving",
            Self::Comment { .. } => "Commenting on",
            Self::RequestChanges { .. } => "Requesting changes on",
            Self::Close { .. } => "Closing",
        }
    }

    /// Get the title for the popup
    pub fn popup_title(&self) -> &'static str {
        match self {
            Self::Approve { .. } => "Approve Pull Request",
            Self::Comment { .. } => "Comment on Pull Request",
            Self::RequestChanges { .. } => "Request Changes",
            Self::Close { .. } => "Close Pull Request",
        }
    }

    /// Get the instruction text
    pub fn instructions(&self) -> &'static str {
        match self {
            Self::Approve { .. } => "Enter your approval message:",
            Self::Comment { .. } => "Enter your comment:",
            Self::RequestChanges { .. } => "Enter your change request message:",
            Self::Close { .. } => "Enter a closing comment (optional):",
        }
    }
}

/// State for the confirmation popup
#[derive(Debug, Clone)]
pub struct ConfirmationPopupState {
    /// The intent - what action to perform on confirm
    pub intent: ConfirmationIntent,
    /// The message input value (user-editable)
    pub input_value: String,
    /// Repository context (owner/repo) for display
    pub repo_context: String,
}

impl ConfirmationPopupState {
    /// Create a new confirmation popup state
    pub fn new(intent: ConfirmationIntent, default_message: String, repo_context: String) -> Self {
        Self {
            intent,
            input_value: default_message,
            repo_context,
        }
    }

    /// Format the target info string (e.g., "PR #123" or "PR #123, #321, #453")
    pub fn target_info(&self) -> String {
        let pr_numbers = self.intent.pr_numbers();
        if pr_numbers.is_empty() {
            return String::new();
        }

        // Format as "PR #123" or "PR #123, #321, #453"
        let mut result = format!("PR #{}", pr_numbers[0]);
        for pr in &pr_numbers[1..] {
            result.push_str(&format!(", #{}", pr));
        }
        result
    }

    /// Get the full title including context
    pub fn title(&self) -> &'static str {
        self.intent.popup_title()
    }

    /// Get the instructions text
    pub fn instructions(&self) -> &'static str {
        self.intent.instructions()
    }

    /// Get the action verb for display
    pub fn action_verb(&self) -> &'static str {
        self.intent.action_verb()
    }

    /// Check if input is required (non-empty) for this action
    pub fn requires_input(&self) -> bool {
        match self.intent {
            // Comment requires a message
            ConfirmationIntent::Comment { .. } => true,
            // Request changes requires a message
            ConfirmationIntent::RequestChanges { .. } => true,
            // Approve and close can have empty messages
            ConfirmationIntent::Approve { .. } | ConfirmationIntent::Close { .. } => false,
        }
    }

    /// Check if the form is valid for submission
    pub fn is_valid(&self) -> bool {
        if self.requires_input() {
            !self.input_value.trim().is_empty()
        } else {
            true
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_info_single_pr() {
        let state = ConfirmationPopupState::new(
            ConfirmationIntent::Approve {
                pr_numbers: vec![123],
            },
            "Test".to_string(),
            "owner/repo".to_string(),
        );
        assert_eq!(state.target_info(), "PR #123");
    }

    #[test]
    fn test_target_info_multiple_prs() {
        let state = ConfirmationPopupState::new(
            ConfirmationIntent::Approve {
                pr_numbers: vec![123, 456, 789],
            },
            "Test".to_string(),
            "owner/repo".to_string(),
        );
        assert_eq!(state.target_info(), "PR #123, #456, #789");
    }

    #[test]
    fn test_requires_input() {
        let approve = ConfirmationIntent::Approve {
            pr_numbers: vec![1],
        };
        let comment = ConfirmationIntent::Comment {
            pr_numbers: vec![1],
        };
        let request = ConfirmationIntent::RequestChanges {
            pr_numbers: vec![1],
        };
        let close = ConfirmationIntent::Close {
            pr_numbers: vec![1],
        };

        let state_approve =
            ConfirmationPopupState::new(approve, String::new(), "owner/repo".to_string());
        let state_comment =
            ConfirmationPopupState::new(comment, String::new(), "owner/repo".to_string());
        let state_request =
            ConfirmationPopupState::new(request, String::new(), "owner/repo".to_string());
        let state_close =
            ConfirmationPopupState::new(close, String::new(), "owner/repo".to_string());

        assert!(!state_approve.requires_input()); // Approve doesn't require message
        assert!(state_comment.requires_input()); // Comment requires message
        assert!(state_request.requires_input()); // Request changes requires message
        assert!(!state_close.requires_input()); // Close doesn't require message
    }

    #[test]
    fn test_is_valid() {
        let comment = ConfirmationIntent::Comment {
            pr_numbers: vec![1],
        };

        let state_empty =
            ConfirmationPopupState::new(comment.clone(), String::new(), "owner/repo".to_string());
        let state_with_msg =
            ConfirmationPopupState::new(comment, "Hello".to_string(), "owner/repo".to_string());

        assert!(!state_empty.is_valid()); // Empty comment is invalid
        assert!(state_with_msg.is_valid()); // Non-empty comment is valid
    }
}

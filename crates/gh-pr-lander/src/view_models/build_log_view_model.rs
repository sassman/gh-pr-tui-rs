//! Build Log View Model
//!
//! Pre-computes presentation data for the build log panel.

use crate::state::{BuildLogJobMetadata, BuildLogJobStatus, BuildLogLoadingState, BuildLogState};
use ratatui::style::Color;

/// View model for rendering the build log panel
#[derive(Debug, Clone)]
pub struct BuildLogViewModel {
    /// PR header information (already formatted)
    pub pr_header: BuildLogPrHeaderViewModel,

    /// Flattened list of visible tree rows, ready to render
    pub rows: Vec<BuildLogTreeRowViewModel>,

    /// Scroll state
    pub scroll_offset: usize,
    #[allow(dead_code)]
    pub viewport_height: usize,

    /// Loading state
    pub is_loading: bool,
    pub error_message: Option<String>,
}

/// PR header view model for build log
#[derive(Debug, Clone)]
pub struct BuildLogPrHeaderViewModel {
    pub number_text: String, // "#123"
    pub title: String,       // "Fix: broken tests"
    pub author_text: String, // "by sassman"
    pub number_color: Color, // theme.status_info
    pub title_color: Color,  // theme.text_primary
    pub author_color: Color, // theme.text_muted
}

/// Tree row view model for build log
#[derive(Debug, Clone)]
pub struct BuildLogTreeRowViewModel {
    /// Complete display text (already formatted with indent, icon, status)
    pub text: String,

    /// Indentation level (for manual indent if needed)
    #[allow(dead_code)]
    pub indent_level: usize,

    /// Whether this row is under cursor
    pub is_cursor: bool,

    /// Pre-determined style
    pub style: BuildLogRowStyle,

    /// Additional metadata for interactions (not displayed)
    #[allow(dead_code)]
    pub path: Vec<usize>,
    #[allow(dead_code)]
    pub node_type: BuildLogNodeType,
}

/// Row styling for build log tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildLogRowStyle {
    Normal,
    Error,   // Red text for errors
    Success, // Green for success
    #[allow(dead_code)]
    Selected, // Highlighted background
}

/// Node type in build log tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildLogNodeType {
    Workflow,
    Job,
    Step,
    LogLine,
}

impl BuildLogViewModel {
    /// Transform BuildLogState into display-ready view model
    pub fn from_state(state: &BuildLogState, theme: &gh_pr_lander_theme::Theme) -> Self {
        let is_loading = matches!(state.loading_state, BuildLogLoadingState::Loading);
        let error_message = match &state.loading_state {
            BuildLogLoadingState::Error(e) => Some(e.clone()),
            _ => None,
        };

        let pr_header = BuildLogPrHeaderViewModel {
            number_text: format!("#{}", state.pr_context.number),
            title: state.pr_context.title.clone(),
            author_text: format!("by {}", state.pr_context.author),
            number_color: theme.status_info,
            title_color: theme.text_primary,
            author_color: theme.text_muted,
        };

        let visible_paths = state.flatten_visible_nodes();
        let mut rows = Vec::new();

        for path in visible_paths.iter() {
            let row = Self::build_row_view_model(state, path);
            rows.push(row);
        }

        Self {
            pr_header,
            rows,
            scroll_offset: state.scroll_offset,
            viewport_height: state.viewport_height,
            is_loading,
            error_message,
        }
    }

    fn build_row_view_model(state: &BuildLogState, path: &[usize]) -> BuildLogTreeRowViewModel {
        let indent_level = path.len().saturating_sub(1);

        // Tree structure alignment:
        // Level 1 (Workflow): "▼ ✗ name"
        // Level 2 (Job):      "├─ ▼ ✗ name"
        // Level 3 (Step):     "│  ├─ ▼ ✗ name"  (│ below ▼ of level 1)
        // Level 4 (Line):     "│  │  content"   (│ below ▼ of levels 1 and 2)

        match path.len() {
            1 => {
                // Workflow node (level 1) - no indent
                let workflow = &state.workflows[path[0]];
                let expanded = state.is_expanded(path);

                let icon = if workflow.jobs.is_empty() {
                    " "
                } else if expanded {
                    "▼"
                } else {
                    "▶"
                };

                let status_icon = if workflow.has_failures {
                    BuildLogJobStatus::Failure.icon()
                } else {
                    BuildLogJobStatus::Success.icon()
                };

                let error_info = if workflow.total_errors > 0 {
                    format!(" ({} errors)", workflow.total_errors)
                } else {
                    String::new()
                };

                let text = format!("{} {} {}{}", icon, status_icon, workflow.name, error_info);

                BuildLogTreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == state.cursor_path,
                    style: if workflow.has_failures {
                        BuildLogRowStyle::Error
                    } else {
                        BuildLogRowStyle::Success
                    },
                    path: path.to_vec(),
                    node_type: BuildLogNodeType::Workflow,
                }
            }

            2 => {
                // Job node (level 2) - tree branch from workflow
                let workflow = &state.workflows[path[0]];
                let job = &workflow.jobs[path[1]];
                let expanded = state.is_expanded(path);

                let icon = if job.steps.is_empty() {
                    " "
                } else if expanded {
                    "▼"
                } else {
                    "▶"
                };

                // Get actual job status from metadata (or infer from error count)
                let key = format!("{}:{}", workflow.name, job.name);
                let status = state
                    .job_metadata
                    .get(&key)
                    .map(|m| m.status)
                    .unwrap_or_else(|| {
                        if job.error_count > 0 {
                            BuildLogJobStatus::Failure
                        } else {
                            BuildLogJobStatus::Success
                        }
                    });

                let status_icon = status.icon();

                let error_info = if job.error_count > 0 {
                    format!(" ({} errors)", job.error_count)
                } else {
                    String::new()
                };

                let duration_info = Self::format_job_duration(&state.job_metadata, workflow, job);

                // ├─ directly below the ▼ of workflow
                let text = format!(
                    "├─ {} {} {}{}{}",
                    icon, status_icon, job.name, error_info, duration_info
                );

                BuildLogTreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == state.cursor_path,
                    style: Self::job_status_style(status),
                    path: path.to_vec(),
                    node_type: BuildLogNodeType::Job,
                }
            }

            3 => {
                // Step node (level 3) - │ below ▼ of workflow, ├─ below ▼ of job
                let workflow = &state.workflows[path[0]];
                let job = &workflow.jobs[path[1]];
                let step = &job.steps[path[2]];
                let expanded = state.is_expanded(path);

                let icon = if step.lines.is_empty() {
                    " "
                } else if expanded {
                    "▼"
                } else {
                    "▶"
                };

                let status_icon = if step.error_count > 0 {
                    BuildLogJobStatus::Failure.icon()
                } else {
                    BuildLogJobStatus::Success.icon()
                };

                let error_info = if step.error_count > 0 {
                    format!(" ({} errors)", step.error_count)
                } else {
                    String::new()
                };

                // │ at position 0 (below ▼), 2 spaces, ├─ at position 3 (below job's ▼)
                let text = format!("│  ├─ {} {}{}{}", icon, status_icon, step.name, error_info);

                BuildLogTreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == state.cursor_path,
                    style: if step.error_count > 0 {
                        BuildLogRowStyle::Error
                    } else {
                        BuildLogRowStyle::Normal
                    },
                    path: path.to_vec(),
                    node_type: BuildLogNodeType::Step,
                }
            }

            4 => {
                // Log line (leaf node - no icon)
                // Level 4: "│  │  content" (│ below workflow's ▼, │ below job's ▼)
                let workflow = &state.workflows[path[0]];
                let job = &workflow.jobs[path[1]];
                let step = &job.steps[path[2]];
                let line = &step.lines[path[3]];

                // Check if this is an error line
                let is_error = if let Some(ref cmd) = line.command {
                    matches!(cmd, gh_actions_log_parser::WorkflowCommand::Error { .. })
                } else {
                    line.display_content.to_lowercase().contains("error:")
                };

                // Tree prefix: │ at position 0, │ at position 3, then 2 spaces for content
                let prefix = "│  │  ";

                // Add timestamp if available
                let timestamp_part = if state.show_timestamps {
                    if let Some(ref timestamp) = line.timestamp {
                        format!("[{}] ", timestamp)
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                // Apply horizontal scroll to content
                let content = if state.horizontal_scroll > 0 {
                    line.display_content
                        .chars()
                        .skip(state.horizontal_scroll)
                        .collect::<String>()
                } else {
                    line.display_content.clone()
                };

                let text = format!("{}{}{}", prefix, timestamp_part, content);

                let style = if is_error {
                    BuildLogRowStyle::Error
                } else {
                    BuildLogRowStyle::Normal
                };

                BuildLogTreeRowViewModel {
                    text,
                    indent_level,
                    is_cursor: path == state.cursor_path,
                    style,
                    path: path.to_vec(),
                    node_type: BuildLogNodeType::LogLine,
                }
            }

            _ => BuildLogTreeRowViewModel {
                text: String::new(),
                indent_level: 0,
                is_cursor: false,
                style: BuildLogRowStyle::Normal,
                path: path.to_vec(),
                node_type: BuildLogNodeType::LogLine,
            },
        }
    }

    /// Format job duration for display
    fn format_job_duration(
        metadata: &std::collections::HashMap<String, BuildLogJobMetadata>,
        workflow: &gh_actions_log_parser::WorkflowNode,
        job: &gh_actions_log_parser::JobNode,
    ) -> String {
        let key = format!("{}:{}", workflow.name, job.name);

        if let Some(meta) = metadata.get(&key) {
            if let Some(duration) = meta.duration {
                let secs = duration.as_secs();
                return if secs >= 60 {
                    format!(" ({}m {}s)", secs / 60, secs % 60)
                } else {
                    format!(" ({}s)", secs)
                };
            }
        }

        String::new()
    }

    /// Get row style for job status
    fn job_status_style(status: BuildLogJobStatus) -> BuildLogRowStyle {
        match status {
            BuildLogJobStatus::Success => BuildLogRowStyle::Success,
            BuildLogJobStatus::Failure => BuildLogRowStyle::Error,
            BuildLogJobStatus::Cancelled | BuildLogJobStatus::Skipped => BuildLogRowStyle::Normal,
            BuildLogJobStatus::InProgress => BuildLogRowStyle::Normal,
            BuildLogJobStatus::Unknown => BuildLogRowStyle::Normal,
        }
    }

    /// Get rows visible in viewport
    #[allow(dead_code)]
    pub fn visible_rows(&self) -> &[BuildLogTreeRowViewModel] {
        let start = self.scroll_offset;
        let end = (start + self.viewport_height).min(self.rows.len());
        if start < self.rows.len() {
            &self.rows[start..end]
        } else {
            &[]
        }
    }
}

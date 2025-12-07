//! Build Log State
//!
//! State for the build log panel that displays CI workflow results.

use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Job execution status for build logs
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BuildLogJobStatus {
    Success,
    Failure,
    Cancelled,
    Skipped,
    InProgress,
    #[default]
    Unknown,
}

impl BuildLogJobStatus {
    pub fn icon(&self) -> &'static str {
        // TODO: Refactor to use centralized icons from utils/icons.rs
        match self {
            Self::Success => "✅",
            Self::Failure => "🚨",
            Self::Cancelled => "🚫",
            Self::Skipped => "⛓️‍💥",
            Self::InProgress => "⏳",
            Self::Unknown => "🚧",
        }
    }

    pub fn is_failed(&self) -> bool {
        matches!(self, Self::Failure)
    }
}

/// Metadata for a build job (from GitHub API)
#[derive(Debug, Clone)]
pub struct BuildLogJobMetadata {
    pub name: String,
    pub workflow_name: String,
    pub status: BuildLogJobStatus,
    pub error_count: usize,
    pub duration: Option<Duration>,
    pub html_url: String,
}

/// PR context for build log header display
#[derive(Debug, Clone, Default)]
pub struct BuildLogPrContext {
    pub number: usize,
    pub title: String,
    pub author: String,
}

/// Loading state for build logs
#[derive(Debug, Clone, Default)]
pub enum BuildLogLoadingState {
    #[default]
    Idle,
    Loading,
    Loaded,
    Error(String),
}

/// Build log panel state
#[derive(Debug, Clone)]
pub struct BuildLogState {
    /// Tree data from parser
    pub workflows: Vec<gh_actions_log_parser::WorkflowNode>,

    /// Job metadata from GitHub API (key: "workflow:job")
    pub job_metadata: HashMap<String, BuildLogJobMetadata>,

    /// Expanded node paths (key: "0" or "0:1" or "0:1:2")
    pub expanded_nodes: HashSet<String>,

    /// Current cursor path [workflow_idx, job_idx?, step_idx?, line_idx?]
    pub cursor_path: Vec<usize>,

    /// Vertical scroll offset
    pub scroll_offset: usize,

    /// Horizontal scroll (for long log lines)
    pub horizontal_scroll: usize,

    /// Show timestamps toggle
    pub show_timestamps: bool,

    /// Viewport height (set during rendering)
    pub viewport_height: usize,

    /// PR context for header
    pub pr_context: BuildLogPrContext,

    /// Loading state
    pub loading_state: BuildLogLoadingState,
}

impl Default for BuildLogState {
    fn default() -> Self {
        Self {
            workflows: Vec::new(),
            job_metadata: HashMap::new(),
            expanded_nodes: HashSet::new(),
            cursor_path: vec![0],
            scroll_offset: 0,
            horizontal_scroll: 0,
            show_timestamps: false,
            viewport_height: 20,
            pr_context: BuildLogPrContext::default(),
            loading_state: BuildLogLoadingState::Idle,
        }
    }
}

impl BuildLogState {
    /// Convert path to string key for expanded_nodes
    pub fn path_to_key(path: &[usize]) -> String {
        path.iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join(":")
    }

    /// Check if a node is expanded
    pub fn is_expanded(&self, path: &[usize]) -> bool {
        self.expanded_nodes.contains(&Self::path_to_key(path))
    }

    /// Toggle expansion state of a node
    pub fn toggle_expanded(&mut self, path: &[usize]) {
        let key = Self::path_to_key(path);
        if self.expanded_nodes.contains(&key) {
            self.expanded_nodes.remove(&key);
        } else {
            self.expanded_nodes.insert(key);
        }
    }

    /// Flatten tree to list of visible node paths
    ///
    /// Returns paths as vectors: \[workflow\], \[workflow, job\], \[workflow, job, step\], etc.
    pub fn flatten_visible_nodes(&self) -> Vec<Vec<usize>> {
        let mut result = Vec::new();

        for (w_idx, workflow) in self.workflows.iter().enumerate() {
            // Workflow node
            result.push(vec![w_idx]);

            if !self.is_expanded(&[w_idx]) {
                continue;
            }

            for (j_idx, job) in workflow.jobs.iter().enumerate() {
                // Job node
                result.push(vec![w_idx, j_idx]);

                if !self.is_expanded(&[w_idx, j_idx]) {
                    continue;
                }

                for (s_idx, step) in job.steps.iter().enumerate() {
                    // Step node
                    result.push(vec![w_idx, j_idx, s_idx]);

                    if !self.is_expanded(&[w_idx, j_idx, s_idx]) {
                        continue;
                    }

                    // Log lines
                    for l_idx in 0..step.lines.len() {
                        result.push(vec![w_idx, j_idx, s_idx, l_idx]);
                    }
                }
            }
        }

        result
    }

    /// Get visible nodes within viewport
    pub fn visible_nodes_in_viewport(&self) -> Vec<Vec<usize>> {
        let all_visible = self.flatten_visible_nodes();
        all_visible
            .into_iter()
            .skip(self.scroll_offset)
            .take(self.viewport_height)
            .collect()
    }

    /// Find index of cursor in flattened visible nodes
    pub fn cursor_index(&self) -> Option<usize> {
        let visible = self.flatten_visible_nodes();
        visible.iter().position(|p| p == &self.cursor_path)
    }

    /// Total number of visible nodes
    pub fn total_visible_nodes(&self) -> usize {
        self.flatten_visible_nodes().len()
    }
}

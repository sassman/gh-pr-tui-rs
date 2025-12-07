//! Build Log Reducer
//!
//! Handles state updates for the build log panel.

use crate::actions::BuildLogAction;
use crate::state::{BuildLogLoadingState, BuildLogState};

/// Reduce build log state based on BuildLogAction
pub fn reduce_build_log(mut state: BuildLogState, action: &BuildLogAction) -> BuildLogState {
    match action {
        BuildLogAction::LoadStart => {
            state.loading_state = BuildLogLoadingState::Loading;
        }

        BuildLogAction::Loaded {
            workflows,
            job_metadata,
            pr_context,
        } => {
            state.workflows = workflows.clone();
            state.pr_context = pr_context.clone();
            state.loading_state = BuildLogLoadingState::Loaded;

            // Build job metadata map
            state.job_metadata.clear();
            for meta in job_metadata {
                let key = format!("{}:{}", meta.workflow_name, meta.name);
                state.job_metadata.insert(key, meta.clone());
            }

            // Auto-expand nodes with errors
            state.expanded_nodes.clear();
            auto_expand_errors(&mut state);

            // Reset cursor
            state.cursor_path = vec![0];
            state.scroll_offset = 0;
        }

        BuildLogAction::LoadError(error) => {
            state.loading_state = BuildLogLoadingState::Error(error.clone());
        }

        BuildLogAction::NavigateDown => {
            build_log_navigate_down(&mut state);
        }

        BuildLogAction::NavigateUp => {
            build_log_navigate_up(&mut state);
        }

        BuildLogAction::NavigateToTop => {
            let visible = state.flatten_visible_nodes();
            if let Some(first) = visible.first() {
                state.cursor_path = first.clone();
                state.scroll_offset = 0;
            }
        }

        BuildLogAction::NavigateToBottom => {
            let visible = state.flatten_visible_nodes();
            if let Some(last) = visible.last() {
                state.cursor_path = last.clone();
                // Scroll to show the last item
                let total = visible.len();
                if total > state.viewport_height {
                    state.scroll_offset = total - state.viewport_height;
                }
            }
        }

        BuildLogAction::Toggle => {
            state.toggle_expanded(&state.cursor_path.clone());
        }

        BuildLogAction::ExpandAll => {
            expand_all(&mut state);
        }

        BuildLogAction::CollapseAll => {
            state.expanded_nodes.clear();
            // Keep cursor at workflow level
            if !state.cursor_path.is_empty() {
                state.cursor_path = vec![state.cursor_path[0]];
            }
            state.scroll_offset = 0;
        }

        BuildLogAction::NextError => {
            build_log_find_next_error(&mut state);
        }

        BuildLogAction::PrevError => {
            build_log_find_prev_error(&mut state);
        }

        BuildLogAction::ScrollLeft => {
            state.horizontal_scroll = state.horizontal_scroll.saturating_sub(4);
        }

        BuildLogAction::ScrollRight => {
            state.horizontal_scroll = state.horizontal_scroll.saturating_add(4);
        }

        BuildLogAction::PageDown => {
            let page_size = state.viewport_height.saturating_sub(2);
            for _ in 0..page_size {
                build_log_navigate_down(&mut state);
            }
        }

        BuildLogAction::PageUp => {
            let page_size = state.viewport_height.saturating_sub(2);
            for _ in 0..page_size {
                build_log_navigate_up(&mut state);
            }
        }

        BuildLogAction::ToggleTimestamps => {
            state.show_timestamps = !state.show_timestamps;
        }

        BuildLogAction::SetViewportHeight(height) => {
            state.viewport_height = *height;
        }

        // Open is handled by middleware, not reducer
        BuildLogAction::Open => {}
    }

    state
}

/// Auto-expand workflows and nodes with errors
fn auto_expand_errors(state: &mut BuildLogState) {
    for (w_idx, workflow) in state.workflows.iter().enumerate() {
        // Always expand workflows (top level)
        state.expanded_nodes.insert(w_idx.to_string());

        // Auto-expand jobs and steps with errors
        for (j_idx, job) in workflow.jobs.iter().enumerate() {
            if job.error_count > 0 {
                state.expanded_nodes.insert(format!("{}:{}", w_idx, j_idx));

                for (s_idx, step) in job.steps.iter().enumerate() {
                    if step.error_count > 0 {
                        state
                            .expanded_nodes
                            .insert(format!("{}:{}:{}", w_idx, j_idx, s_idx));
                    }
                }
            }
        }
    }
}

/// Expand all nodes in the tree
fn expand_all(state: &mut BuildLogState) {
    for (w_idx, workflow) in state.workflows.iter().enumerate() {
        state.expanded_nodes.insert(w_idx.to_string());

        for (j_idx, job) in workflow.jobs.iter().enumerate() {
            state.expanded_nodes.insert(format!("{}:{}", w_idx, j_idx));

            for (s_idx, _step) in job.steps.iter().enumerate() {
                state
                    .expanded_nodes
                    .insert(format!("{}:{}:{}", w_idx, j_idx, s_idx));
            }
        }
    }
}

/// Navigate down to next visible tree node
fn build_log_navigate_down(state: &mut BuildLogState) {
    let visible = state.flatten_visible_nodes();
    if visible.is_empty() {
        return;
    }

    // Find current position in flattened list
    if let Some(current_idx) = visible.iter().position(|path| path == &state.cursor_path) {
        if current_idx < visible.len() - 1 {
            let new_idx = current_idx + 1;
            state.cursor_path = visible[new_idx].clone();

            // Auto-scroll to keep cursor visible
            let max_visible_idx = state.scroll_offset + state.viewport_height.saturating_sub(1);
            if new_idx > max_visible_idx {
                state.scroll_offset =
                    new_idx.saturating_sub(state.viewport_height.saturating_sub(1));
            }
        }
    }
}

/// Navigate up to previous visible tree node
fn build_log_navigate_up(state: &mut BuildLogState) {
    let visible = state.flatten_visible_nodes();
    if visible.is_empty() {
        return;
    }

    // Find current position in flattened list
    if let Some(current_idx) = visible.iter().position(|path| path == &state.cursor_path) {
        if current_idx > 0 {
            let new_idx = current_idx - 1;
            state.cursor_path = visible[new_idx].clone();

            // Auto-scroll to keep cursor visible
            if new_idx < state.scroll_offset {
                state.scroll_offset = new_idx;
            }
        }
    }
}

/// Collect all tree paths that have errors
fn collect_error_paths(state: &BuildLogState) -> Vec<Vec<usize>> {
    let mut result = Vec::new();

    for (w_idx, workflow) in state.workflows.iter().enumerate() {
        for (j_idx, job) in workflow.jobs.iter().enumerate() {
            if job.error_count > 0 {
                result.push(vec![w_idx, j_idx]);
            }

            for (s_idx, step) in job.steps.iter().enumerate() {
                if step.error_count > 0 {
                    result.push(vec![w_idx, j_idx, s_idx]);
                }
            }
        }
    }

    result
}

/// Check if a log line is an error
fn is_error_line(line: &gh_actions_log_parser::LogLine) -> bool {
    if let Some(ref cmd) = line.command {
        matches!(cmd, gh_actions_log_parser::WorkflowCommand::Error { .. })
    } else {
        line.display_content.to_lowercase().contains("error:")
    }
}

/// Find next error across entire tree
fn build_log_find_next_error(state: &mut BuildLogState) {
    // Check if we're in a step (path length 3) or at a log line (path length 4)
    if state.cursor_path.len() >= 3 {
        let step_path = &state.cursor_path[0..3];

        // Get the step to check for error lines
        if let Some(workflow) = state.workflows.get(step_path[0]) {
            if let Some(job) = workflow.jobs.get(step_path[1]) {
                if let Some(step) = job.steps.get(step_path[2]) {
                    // Check if step is expanded (has visible lines)
                    if state.is_expanded(step_path) {
                        let start_line_idx = if state.cursor_path.len() == 4 {
                            state.cursor_path[3] + 1
                        } else {
                            0
                        };

                        // Find next error line in this step
                        for (line_idx, line) in step.lines.iter().enumerate().skip(start_line_idx) {
                            if is_error_line(line) {
                                let new_path =
                                    vec![step_path[0], step_path[1], step_path[2], line_idx];
                                let visible = state.flatten_visible_nodes();
                                if let Some(idx) = visible.iter().position(|path| path == &new_path)
                                {
                                    state.cursor_path = new_path;
                                    ensure_cursor_visible(state, idx);
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    // No more error lines in current step, jump to next step/job with errors
    let error_paths = collect_error_paths(state);
    if error_paths.is_empty() {
        return;
    }

    let visible = state.flatten_visible_nodes();
    if let Some(current_idx) = visible.iter().position(|path| path == &state.cursor_path) {
        // Look for next error path after current position
        for (idx, path) in visible.iter().enumerate().skip(current_idx + 1) {
            if error_paths.contains(path) {
                state.cursor_path = path.clone();
                ensure_cursor_visible(state, idx);
                return;
            }
        }
    }

    // Wrap to first error
    if let Some(first_error) = error_paths.first() {
        if let Some(idx) = visible.iter().position(|path| path == first_error) {
            state.cursor_path = first_error.clone();
            state.scroll_offset = idx;
        }
    }
}

/// Find previous error across entire tree
fn build_log_find_prev_error(state: &mut BuildLogState) {
    // Check if we're in a step (path length 3) or at a log line (path length 4)
    if state.cursor_path.len() >= 3 {
        let step_path = &state.cursor_path[0..3];

        // Get the step to check for error lines
        if let Some(workflow) = state.workflows.get(step_path[0]) {
            if let Some(job) = workflow.jobs.get(step_path[1]) {
                if let Some(step) = job.steps.get(step_path[2]) {
                    // Check if step is expanded (has visible lines)
                    if state.is_expanded(step_path) {
                        let end_line_idx = if state.cursor_path.len() == 4 {
                            state.cursor_path[3]
                        } else {
                            step.lines.len()
                        };

                        // Find previous error line in this step (iterate backwards)
                        for (line_idx, line) in
                            step.lines.iter().enumerate().take(end_line_idx).rev()
                        {
                            if is_error_line(line) {
                                let new_path =
                                    vec![step_path[0], step_path[1], step_path[2], line_idx];
                                let visible = state.flatten_visible_nodes();
                                if let Some(idx) = visible.iter().position(|path| path == &new_path)
                                {
                                    state.cursor_path = new_path;
                                    ensure_cursor_visible(state, idx);
                                }
                                return;
                            }
                        }
                    }
                }
            }
        }
    }

    // No more error lines in current step, jump to previous step/job with errors
    let error_paths = collect_error_paths(state);
    if error_paths.is_empty() {
        return;
    }

    let visible = state.flatten_visible_nodes();
    if let Some(current_idx) = visible.iter().position(|path| path == &state.cursor_path) {
        // Look for previous error path before current position
        for (idx, path) in visible.iter().enumerate().take(current_idx).rev() {
            if error_paths.contains(path) {
                state.cursor_path = path.clone();
                ensure_cursor_visible(state, idx);
                return;
            }
        }
    }

    // Wrap to last error
    if let Some(last_error) = error_paths.last() {
        if let Some(idx) = visible.iter().position(|path| path == last_error) {
            state.cursor_path = last_error.clone();
            ensure_cursor_visible(state, idx);
        }
    }
}

/// Ensure cursor is visible in viewport by adjusting scroll offset
fn ensure_cursor_visible(state: &mut BuildLogState, cursor_idx: usize) {
    let max_visible_idx = state.scroll_offset + state.viewport_height.saturating_sub(1);
    if cursor_idx > max_visible_idx {
        state.scroll_offset = cursor_idx.saturating_sub(state.viewport_height.saturating_sub(1));
    } else if cursor_idx < state.scroll_offset {
        state.scroll_offset = cursor_idx;
    }
}

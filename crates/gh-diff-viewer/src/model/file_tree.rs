//! File tree model for navigation in the diff viewer.

use super::{FileDiff, FileStatus};

/// Node in the file tree for navigation.
#[derive(Debug, Clone)]
pub struct FileTreeNode {
    /// Display name (file or directory name).
    pub name: String,
    /// Full path (for files, None for directories).
    pub path: Option<String>,
    /// Child nodes (for directories).
    pub children: Vec<FileTreeNode>,
    /// Whether this directory is expanded.
    pub expanded: bool,
    /// File status (for files).
    pub status: Option<FileStatus>,
    /// Number of additions (for files).
    pub additions: usize,
    /// Number of deletions (for files).
    pub deletions: usize,
}

impl FileTreeNode {
    /// Create a new directory node.
    pub fn directory(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: None,
            children: Vec::new(),
            expanded: true,
            status: None,
            additions: 0,
            deletions: 0,
        }
    }

    /// Create a new file node.
    pub fn file(name: impl Into<String>, path: impl Into<String>, file_diff: &FileDiff) -> Self {
        Self {
            name: name.into(),
            path: Some(path.into()),
            children: Vec::new(),
            expanded: false,
            status: Some(file_diff.status),
            additions: file_diff.additions,
            deletions: file_diff.deletions,
        }
    }

    /// Build a file tree from a flat list of FileDiffs.
    pub fn from_files(files: &[FileDiff]) -> Self {
        let mut root = FileTreeNode::directory("");

        for file in files {
            let parts: Vec<&str> = file.path.split('/').collect();
            root.insert_path(&parts, file);
        }

        // Sort children: directories first, then files, both alphabetically
        root.sort_recursive();
        root
    }

    /// Insert a file path into the tree.
    fn insert_path(&mut self, parts: &[&str], file_diff: &FileDiff) {
        if parts.is_empty() {
            return;
        }

        if parts.len() == 1 {
            // Leaf node (file)
            self.children
                .push(FileTreeNode::file(parts[0], &file_diff.path, file_diff));
        } else {
            // Find or create directory
            let dir_name = parts[0];
            let child = self
                .children
                .iter_mut()
                .find(|c| c.name == dir_name && c.path.is_none());

            if let Some(dir) = child {
                dir.insert_path(&parts[1..], file_diff);
            } else {
                let mut new_dir = FileTreeNode::directory(dir_name);
                new_dir.insert_path(&parts[1..], file_diff);
                self.children.push(new_dir);
            }
        }
    }

    /// Sort children recursively (directories first, then alphabetically).
    fn sort_recursive(&mut self) {
        self.children.sort_by(|a, b| {
            let a_is_dir = a.path.is_none();
            let b_is_dir = b.path.is_none();
            match (a_is_dir, b_is_dir) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => a.name.cmp(&b.name),
            }
        });

        for child in &mut self.children {
            child.sort_recursive();
        }
    }

    /// Check if this node is a directory.
    pub fn is_directory(&self) -> bool {
        self.path.is_none()
    }

    /// Toggle expanded state (for directories).
    pub fn toggle(&mut self) {
        if self.is_directory() {
            self.expanded = !self.expanded;
        }
    }

    /// Flatten the tree into a list for rendering (respecting expanded state).
    pub fn flatten(&self) -> Vec<FlatFileEntry> {
        let mut result = Vec::new();
        self.flatten_recursive(0, &mut result, &[]);
        result
    }

    fn flatten_recursive(
        &self,
        depth: usize,
        result: &mut Vec<FlatFileEntry>,
        ancestor_has_next: &[bool],
    ) {
        // Skip root node itself
        if !self.name.is_empty() {
            result.push(FlatFileEntry {
                depth,
                name: self.name.clone(),
                path: self.path.clone(),
                is_dir: self.is_directory(),
                is_expanded: self.expanded,
                status: self.status,
                additions: self.additions,
                deletions: self.deletions,
                is_last: false, // Will be set by parent
                ancestor_has_next: ancestor_has_next.to_vec(),
            });
        }

        // Add children if directory is expanded (or if this is root)
        if self.expanded || self.name.is_empty() {
            let child_depth = if self.name.is_empty() { 0 } else { depth + 1 };
            let child_count = self.children.len();

            for (i, child) in self.children.iter().enumerate() {
                let is_last = i == child_count - 1;

                // Build ancestor_has_next for children
                let mut child_ancestor_has_next = ancestor_has_next.to_vec();
                // Only add entry if this is not root
                if !self.name.is_empty() {
                    // This node has more siblings if it's not the last
                    child_ancestor_has_next.push(!is_last);
                }

                child.flatten_recursive(child_depth, result, &child_ancestor_has_next);

                // Mark the entry we just added as last if applicable
                if is_last {
                    // Find the entry we just added for this child
                    if let Some(entry) = result.iter_mut().rev().find(|e| {
                        e.name == child.name && e.depth == child_depth && e.path == child.path
                    }) {
                        entry.is_last = true;
                    }
                }
            }
        }
    }

    /// Find a node by path and toggle its expanded state.
    pub fn toggle_at_path(&mut self, target_path: &str) -> bool {
        if self.path.as_deref() == Some(target_path) {
            return false; // Files can't be toggled
        }

        // Check if this directory's "virtual path" matches
        // For now, just recurse and toggle matching dirs by name
        for child in &mut self.children {
            if child.is_directory() && child.name == target_path {
                child.toggle();
                return true;
            }
            if child.toggle_at_path(target_path) {
                return true;
            }
        }
        false
    }

    /// Calculate aggregate stats for directories.
    pub fn calculate_stats(&mut self) -> (usize, usize) {
        if !self.is_directory() {
            return (self.additions, self.deletions);
        }

        let mut total_add = 0;
        let mut total_del = 0;
        for child in &mut self.children {
            let (add, del) = child.calculate_stats();
            total_add += add;
            total_del += del;
        }
        self.additions = total_add;
        self.deletions = total_del;
        (total_add, total_del)
    }

    /// Get file paths in display order.
    pub fn file_paths(&self) -> Vec<String> {
        self.flatten().into_iter().filter_map(|e| e.path).collect()
    }
}

/// A flattened file tree entry for rendering.
#[derive(Debug, Clone)]
pub struct FlatFileEntry {
    /// Nesting depth (0 = top level).
    pub depth: usize,
    /// Display name (file or directory name).
    pub name: String,
    /// Full path (for files, None for directories).
    pub path: Option<String>,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether this directory is expanded.
    pub is_expanded: bool,
    /// File status (for files).
    pub status: Option<FileStatus>,
    /// Number of additions.
    pub additions: usize,
    /// Number of deletions.
    pub deletions: usize,
    /// Whether this is the last item in its parent.
    pub is_last: bool,
    /// For each ancestor level, whether that ancestor has more siblings below.
    /// Used to determine where to draw vertical continuation lines (│).
    pub ancestor_has_next: Vec<bool>,
}

impl FlatFileEntry {
    /// Get the icon for this entry.
    pub fn icon(&self) -> &'static str {
        if self.is_dir {
            if self.is_expanded {
                "▼ "
            } else {
                "▶ "
            }
        } else {
            "  "
        }
    }

    /// Get indent string based on depth (legacy, use tree_prefix for tree lines).
    pub fn indent(&self) -> String {
        "  ".repeat(self.depth)
    }

    /// Get the tree prefix with guide lines.
    /// Example output for different positions:
    /// - Top level: ""
    /// - First child of root: "├─ "
    /// - Last child of root: "└─ "
    /// - Nested with siblings above: "│  ├─ "
    /// - Nested last item: "│  └─ "
    pub fn tree_prefix(&self) -> String {
        if self.depth == 0 {
            return String::new();
        }

        let mut prefix = String::new();

        // Add continuation lines for ancestors
        for &has_next in &self.ancestor_has_next {
            if has_next {
                prefix.push_str("│  ");
            } else {
                prefix.push_str("   ");
            }
        }

        // Add branch for this node
        if self.is_last {
            prefix.push_str("└─ ");
        } else {
            prefix.push_str("├─ ");
        }

        prefix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_file_diff(path: &str, additions: usize, deletions: usize) -> FileDiff {
        let mut f = FileDiff::new(path);
        f.additions = additions;
        f.deletions = deletions;
        f
    }

    #[test]
    fn test_file_tree_construction() {
        let files = vec![
            make_file_diff("src/main.rs", 10, 5),
            make_file_diff("src/lib.rs", 3, 1),
            make_file_diff("tests/test.rs", 20, 0),
            make_file_diff("Cargo.toml", 2, 1),
        ];

        let tree = FileTreeNode::from_files(&files);
        let flat = tree.flatten();

        // Should have: src/, src/lib.rs, src/main.rs, tests/, tests/test.rs, Cargo.toml
        assert_eq!(flat.len(), 6);

        // Directories come first
        assert!(flat[0].is_dir);
        assert_eq!(flat[0].name, "src");

        // Files in src are sorted
        assert!(!flat[1].is_dir);
        assert_eq!(flat[1].name, "lib.rs");
    }

    #[test]
    fn test_file_tree_collapse() {
        let files = vec![
            make_file_diff("src/main.rs", 10, 5),
            make_file_diff("src/lib.rs", 3, 1),
        ];

        let mut tree = FileTreeNode::from_files(&files);

        // Initially expanded
        let flat = tree.flatten();
        assert_eq!(flat.len(), 3); // src/, lib.rs, main.rs

        // Collapse src
        tree.children[0].toggle();
        let flat = tree.flatten();
        assert_eq!(flat.len(), 1); // just src/
    }

    #[test]
    fn test_flat_entry_icon() {
        let dir = FlatFileEntry {
            depth: 0,
            name: "src".to_string(),
            path: None,
            is_dir: true,
            is_expanded: true,
            status: None,
            additions: 0,
            deletions: 0,
            is_last: false,
            ancestor_has_next: vec![],
        };
        assert_eq!(dir.icon(), "▼ ");

        let collapsed_dir = FlatFileEntry {
            is_expanded: false,
            ..dir.clone()
        };
        assert_eq!(collapsed_dir.icon(), "▶ ");

        let file = FlatFileEntry {
            is_dir: false,
            path: Some("src/main.rs".to_string()),
            ..dir
        };
        assert_eq!(file.icon(), "  ");
    }

    #[test]
    fn test_tree_prefix() {
        // Top level directory - no prefix
        let top_level = FlatFileEntry {
            depth: 0,
            name: "src".to_string(),
            path: None,
            is_dir: true,
            is_expanded: true,
            status: None,
            additions: 0,
            deletions: 0,
            is_last: false,
            ancestor_has_next: vec![],
        };
        assert_eq!(top_level.tree_prefix(), "");

        // First child (not last) - uses ├─
        let first_child = FlatFileEntry {
            depth: 1,
            name: "lib.rs".to_string(),
            path: Some("src/lib.rs".to_string()),
            is_dir: false,
            is_expanded: false,
            status: None,
            additions: 0,
            deletions: 0,
            is_last: false,
            ancestor_has_next: vec![],
        };
        assert_eq!(first_child.tree_prefix(), "├─ ");

        // Last child - uses └─
        let last_child = FlatFileEntry {
            is_last: true,
            ..first_child.clone()
        };
        assert_eq!(last_child.tree_prefix(), "└─ ");

        // Nested child with ancestor that has more siblings
        let nested = FlatFileEntry {
            depth: 2,
            name: "mod.rs".to_string(),
            path: Some("src/utils/mod.rs".to_string()),
            is_dir: false,
            is_expanded: false,
            status: None,
            additions: 0,
            deletions: 0,
            is_last: true,
            ancestor_has_next: vec![true], // parent (utils/) has siblings
        };
        assert_eq!(nested.tree_prefix(), "│  └─ ");

        // Nested child with ancestor that has no more siblings
        let nested_no_sibling = FlatFileEntry {
            ancestor_has_next: vec![false], // parent is last in its group
            ..nested.clone()
        };
        assert_eq!(nested_no_sibling.tree_prefix(), "   └─ ");
    }
}

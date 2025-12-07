//! Parse unified diff format (as returned by GitHub API).

use crate::model::{DiffLine, FileDiff, FileStatus, Hunk, LineKind, PullRequestDiff};
use thiserror::Error;
use unidiff::{Hunk as UnidiffHunk, Line as UnidiffLine, PatchSet, PatchedFile};

/// Errors that can occur during diff parsing.
#[derive(Debug, Error)]
pub enum ParseError {
    #[error("Failed to parse diff: {0}")]
    ParseFailed(String),
    #[error("Invalid diff format")]
    InvalidFormat,
}

/// Parse a unified diff string into a structured `PullRequestDiff`.
///
/// # Arguments
/// * `diff_text` - The unified diff text (from GitHub API or git diff)
/// * `base_sha` - The base commit SHA
/// * `head_sha` - The head commit SHA
///
/// # Example
/// ```ignore
/// let diff = parse_unified_diff(diff_text, "abc123", "def456")?;
/// println!("Changed files: {}", diff.files.len());
/// ```
pub fn parse_unified_diff(
    diff_text: &str,
    base_sha: impl Into<String>,
    head_sha: impl Into<String>,
) -> Result<PullRequestDiff, ParseError> {
    let mut patch_set = PatchSet::new();
    patch_set
        .parse(diff_text)
        .map_err(|e| ParseError::ParseFailed(e.to_string()))?;

    let mut diff = PullRequestDiff::new(base_sha, head_sha);

    for patched_file in patch_set.files() {
        let file_diff = parse_patched_file(patched_file)?;
        diff.files.push(file_diff);
    }

    diff.recalculate_totals();
    Ok(diff)
}

fn parse_patched_file(file: &PatchedFile) -> Result<FileDiff, ParseError> {
    let target = clean_path(&file.target_file);
    let source = clean_path(&file.source_file);

    let mut file_diff = FileDiff::new(&target);

    // Determine file status
    file_diff.status = determine_status(&source, &target, file);

    // Set old path if different
    if source != target && !source.is_empty() && source != "/dev/null" {
        file_diff.old_path = Some(source);
    }

    // Parse hunks
    for hunk in file.hunks() {
        file_diff.hunks.push(parse_hunk(hunk)?);
    }

    file_diff.recalculate_stats();
    Ok(file_diff)
}

fn parse_hunk(hunk: &UnidiffHunk) -> Result<Hunk, ParseError> {
    let mut parsed = Hunk::new(
        hunk.source_start as u32,
        hunk.source_length as u32,
        hunk.target_start as u32,
        hunk.target_length as u32,
    );

    // Extract function context from section header if available
    let header = &hunk.section_header;
    if !header.is_empty() {
        parsed.header = format!(
            "@@ -{},{} +{},{} @@ {}",
            parsed.old_start, parsed.old_count, parsed.new_start, parsed.new_count, header
        );
    }

    // Parse lines
    for line in hunk.lines() {
        parsed.lines.push(parse_line(line)?);
    }

    Ok(parsed)
}

fn parse_line(line: &UnidiffLine) -> Result<DiffLine, ParseError> {
    let content = line.value.to_string();
    let source_line = line.source_line_no.map(|n| n as u32);
    let target_line = line.target_line_no.map(|n| n as u32);

    let kind = match line.line_type.as_str() {
        " " => LineKind::Context,
        "+" => LineKind::Addition,
        "-" => LineKind::Deletion,
        "\\" => LineKind::Context, // "\ No newline at end of file"
        _ => LineKind::Context,
    };

    Ok(DiffLine {
        kind,
        content,
        old_line: source_line,
        new_line: target_line,
        is_expanded: false,
        highlighted: None,
    })
}

fn determine_status(source: &str, target: &str, _file: &PatchedFile) -> FileStatus {
    if source == "/dev/null" || source.is_empty() {
        FileStatus::Added
    } else if target == "/dev/null" || target.is_empty() {
        FileStatus::Deleted
    } else if source != target {
        FileStatus::Renamed
    } else {
        FileStatus::Modified
    }
}

/// Clean the path by removing a/b prefixes from git diff output.
fn clean_path(path: &str) -> String {
    let path = path.trim();

    // Remove common prefixes
    if let Some(stripped) = path.strip_prefix("a/") {
        return stripped.to_string();
    }
    if let Some(stripped) = path.strip_prefix("b/") {
        return stripped.to_string();
    }

    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_DIFF: &str = r#"diff --git a/src/main.rs b/src/main.rs
index abc123..def456 100644
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,5 +1,6 @@ fn main()
 fn main() {
     println!("Hello");
+    println!("World");
 }

diff --git a/src/lib.rs b/src/lib.rs
index 111222..333444 100644
--- a/src/lib.rs
+++ b/src/lib.rs
@@ -10,7 +10,6 @@ impl Foo {
 impl Foo {
     fn bar(&self) {
-        // old comment
         self.do_thing();
     }
 }
"#;

    #[test]
    fn test_parse_simple_diff() {
        let diff = parse_unified_diff(SAMPLE_DIFF, "abc", "def").unwrap();

        assert_eq!(diff.files.len(), 2);
        assert_eq!(diff.total_additions, 1);
        assert_eq!(diff.total_deletions, 1);

        // First file
        let file1 = &diff.files[0];
        assert_eq!(file1.path, "src/main.rs");
        assert_eq!(file1.status, FileStatus::Modified);
        assert_eq!(file1.additions, 1);
        assert_eq!(file1.deletions, 0);
        assert_eq!(file1.hunks.len(), 1);

        // Check hunk
        let hunk = &file1.hunks[0];
        assert_eq!(hunk.old_start, 1);
        assert_eq!(hunk.new_start, 1);
        assert!(hunk.header.contains("fn main()"));

        // Second file
        let file2 = &diff.files[1];
        assert_eq!(file2.path, "src/lib.rs");
        assert_eq!(file2.additions, 0);
        assert_eq!(file2.deletions, 1);
    }

    #[test]
    fn test_parse_new_file() {
        let diff = r#"diff --git a/new_file.rs b/new_file.rs
new file mode 100644
index 0000000..abc1234
--- /dev/null
+++ b/new_file.rs
@@ -0,0 +1,3 @@
+fn new_function() {
+    // new code
+}
"#;

        let parsed = parse_unified_diff(diff, "base", "head").unwrap();
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].status, FileStatus::Added);
        assert_eq!(parsed.files[0].additions, 3);
    }

    #[test]
    fn test_parse_deleted_file() {
        let diff = r#"diff --git a/old_file.rs b/old_file.rs
deleted file mode 100644
index abc1234..0000000
--- a/old_file.rs
+++ /dev/null
@@ -1,3 +0,0 @@
-fn old_function() {
-    // old code
-}
"#;

        let parsed = parse_unified_diff(diff, "base", "head").unwrap();
        assert_eq!(parsed.files.len(), 1);
        assert_eq!(parsed.files[0].status, FileStatus::Deleted);
        assert_eq!(parsed.files[0].deletions, 3);
    }

    #[test]
    fn test_parse_renamed_file() {
        let diff = r#"diff --git a/old_name.rs b/new_name.rs
similarity index 95%
rename from old_name.rs
rename to new_name.rs
index abc123..def456 100644
--- a/old_name.rs
+++ b/new_name.rs
@@ -1,3 +1,3 @@
 fn example() {
-    // old
+    // new
 }
"#;

        let parsed = parse_unified_diff(diff, "base", "head").unwrap();
        assert_eq!(parsed.files.len(), 1);

        let file = &parsed.files[0];
        assert_eq!(file.path, "new_name.rs");
        assert_eq!(file.old_path, Some("old_name.rs".to_string()));
        assert_eq!(file.status, FileStatus::Renamed);
    }

    #[test]
    fn test_clean_path() {
        assert_eq!(clean_path("a/src/main.rs"), "src/main.rs");
        assert_eq!(clean_path("b/src/main.rs"), "src/main.rs");
        assert_eq!(clean_path("src/main.rs"), "src/main.rs");
        assert_eq!(clean_path("/dev/null"), "/dev/null");
    }

    #[test]
    fn test_line_numbers() {
        let diff = parse_unified_diff(SAMPLE_DIFF, "base", "head").unwrap();
        let hunk = &diff.files[0].hunks[0];

        // First line is context: "fn main() {"
        assert_eq!(hunk.lines[0].kind, LineKind::Context);
        assert_eq!(hunk.lines[0].old_line, Some(1));
        assert_eq!(hunk.lines[0].new_line, Some(1));

        // Addition line
        let addition = hunk
            .lines
            .iter()
            .find(|l| l.kind == LineKind::Addition)
            .unwrap();
        assert!(addition.old_line.is_none());
        assert!(addition.new_line.is_some());
    }
}

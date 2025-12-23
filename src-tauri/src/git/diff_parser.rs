//! Unified diff parser for `git diff` output.

use log::warn;

use super::types::{DiffHunk, DiffLine, LineKind};

pub fn parse_unified_diff(diff_output: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_line_no: u32 = 0;
    let mut new_line_no: u32 = 0;

    for line in diff_output.lines() {
        // Skip diff header lines
        if line.starts_with("diff --git")
            || line.starts_with("index ")
            || line.starts_with("--- ")
            || line.starts_with("+++ ")
        {
            continue;
        }

        // @@ -old_start,old_lines +new_start,new_lines @@
        if line.starts_with("@@") {
            if let Some(hunk) = current_hunk.take() {
                hunks.push(hunk);
            }

            if let Some((old_start, old_lines, new_start, new_lines)) = parse_hunk_header(line) {
                old_line_no = old_start;
                new_line_no = new_start;
                current_hunk = Some(DiffHunk {
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    lines: Vec::new(),
                });
            } else {
                warn!("Failed to parse hunk header: {}", line);
            }
            continue;
        }

        if let Some(ref mut hunk) = current_hunk {
            if let Some(first_char) = line.chars().next() {
                let (kind, content) = match first_char {
                    '+' => (LineKind::Addition, &line[1..]),
                    '-' => (LineKind::Deletion, &line[1..]),
                    ' ' => (LineKind::Context, &line[1..]),
                    '\\' => continue, // "\ No newline at end of file"
                    _ => {
                        warn!(
                            "Unexpected diff line format (treating as context): {}",
                            line
                        );
                        (LineKind::Context, line)
                    }
                };

                let (old_no, new_no) = match kind {
                    LineKind::Addition => {
                        let n = new_line_no;
                        new_line_no += 1;
                        (None, Some(n))
                    }
                    LineKind::Deletion => {
                        let n = old_line_no;
                        old_line_no += 1;
                        (Some(n), None)
                    }
                    LineKind::Context => {
                        let (o, n) = (old_line_no, new_line_no);
                        old_line_no += 1;
                        new_line_no += 1;
                        (Some(o), Some(n))
                    }
                };

                hunk.lines.push(DiffLine {
                    kind,
                    old_line_no: old_no,
                    new_line_no: new_no,
                    content: content.to_string(),
                });
            }
        }
    }

    if let Some(hunk) = current_hunk {
        hunks.push(hunk);
    }

    hunks
}

/// Parses "@@ -1,5 +1,6 @@" or "@@ -1 +1,2 @@"
fn parse_hunk_header(line: &str) -> Option<(u32, u32, u32, u32)> {
    let line = line.trim_start_matches("@@ ").trim_end_matches(" @@");
    let line = line.split(" @@").next()?; // Strip trailing context like "fn main()"

    let parts: Vec<&str> = line.split_whitespace().collect();

    if parts.len() < 2 {
        return None;
    }

    let old_part = parts[0].trim_start_matches('-');
    let new_part = parts[1].trim_start_matches('+');

    let (old_start, old_lines) = parse_range(old_part)?;
    let (new_start, new_lines) = parse_range(new_part)?;

    Some((old_start, old_lines, new_start, new_lines))
}

/// Parses "1,5" or "1" into (start, count)
fn parse_range(range: &str) -> Option<(u32, u32)> {
    if let Some((start, count)) = range.split_once(',') {
        Some((start.parse().ok()?, count.parse().ok()?))
    } else {
        Some((range.parse().ok()?, 1)) // Single number = count of 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_diff() {
        let diff = r#"diff --git a/test.txt b/test.txt
index abc123..def456 100644
--- a/test.txt
+++ b/test.txt
@@ -1,3 +1,4 @@
 line1
+added line
 line2
 line3
"#;
        let hunks = parse_unified_diff(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[0].old_lines, 3);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[0].new_lines, 4);
        assert_eq!(hunks[0].lines.len(), 4);
    }

    #[test]
    fn test_parse_hunk_header_with_context() {
        // Some git versions include function context after @@
        let header = "@@ -10,5 +10,6 @@ fn main()";
        let result = parse_hunk_header(header);
        assert_eq!(result, Some((10, 5, 10, 6)));
    }

    #[test]
    fn test_parse_hunk_header_single_line() {
        let header = "@@ -1 +1,2 @@";
        let result = parse_hunk_header(header);
        assert_eq!(result, Some((1, 1, 1, 2)));
    }

    #[test]
    fn test_parse_deletion() {
        let diff = r#"@@ -1,2 +1 @@
 kept
-deleted
"#;
        let hunks = parse_unified_diff(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].lines.len(), 2);
        assert_eq!(hunks[0].lines[1].kind, LineKind::Deletion);
    }
}

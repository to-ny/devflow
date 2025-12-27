use std::fs;
use std::path::Path;
use std::time::SystemTime;

const AGENTS_MD_FILENAME: &str = "AGENTS.md";
const MAX_CHAR_COUNT: usize = 50_000;

#[derive(Debug, Default)]
pub struct MemoryState {
    content: Option<String>,
    char_count: usize,
    truncated: bool,
    last_modified: Option<SystemTime>,
}

#[derive(Debug)]
pub enum LoadResult {
    Loaded {
        path: String,
        char_count: u32,
        truncated: bool,
    },
    NotFound,
    Error(String),
}

impl MemoryState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn load(project_path: &Path) -> (Self, LoadResult) {
        let file_path = project_path.join(AGENTS_MD_FILENAME);

        if !file_path.exists() {
            return (Self::new(), LoadResult::NotFound);
        }

        let metadata = match fs::metadata(&file_path) {
            Ok(m) => m,
            Err(e) => {
                return (
                    Self::new(),
                    LoadResult::Error(format!("Cannot access {}: {}", AGENTS_MD_FILENAME, e)),
                );
            }
        };

        let last_modified = metadata.modified().ok();

        let content = match fs::read_to_string(&file_path) {
            Ok(c) => c,
            Err(e) => {
                return (
                    Self::new(),
                    LoadResult::Error(format!("Cannot read {}: {}", AGENTS_MD_FILENAME, e)),
                );
            }
        };

        let (final_content, truncated) = if content.chars().count() > MAX_CHAR_COUNT {
            let truncate_at = content
                .char_indices()
                .nth(MAX_CHAR_COUNT)
                .map(|(i, _)| i)
                .unwrap_or(content.len());
            let truncated_content = format!(
                "{}\n\n[Content truncated at {} characters]",
                &content[..truncate_at],
                MAX_CHAR_COUNT
            );
            (truncated_content, true)
        } else {
            (content, false)
        };

        let char_count = final_content.len();
        let path_str = file_path.to_string_lossy().to_string();

        let state = Self {
            content: Some(final_content),
            char_count,
            truncated,
            last_modified,
        };

        (
            state,
            LoadResult::Loaded {
                path: path_str,
                char_count: char_count as u32,
                truncated,
            },
        )
    }

    /// Returns Some(LoadResult) if file changed and was reloaded.
    pub fn reload_if_changed(&mut self, project_path: &Path) -> Option<LoadResult> {
        let file_path = project_path.join(AGENTS_MD_FILENAME);

        if !file_path.exists() {
            if self.content.is_some() {
                *self = Self::new();
                return Some(LoadResult::NotFound);
            }
            return None;
        }

        let current_modified = fs::metadata(&file_path)
            .ok()
            .and_then(|m| m.modified().ok());

        let needs_reload = match (current_modified, self.last_modified) {
            (Some(current), Some(last)) => current != last,
            (Some(_), None) => true,
            (None, _) => false,
        };

        if !needs_reload {
            return None;
        }

        let (new_state, result) = Self::load(project_path);
        *self = new_state;
        Some(result)
    }

    pub fn format_for_injection(&self) -> Option<String> {
        self.content.as_ref().map(|content| {
            format!(
                "<project-memory source=\"{}\">\n{}\n</project-memory>",
                AGENTS_MD_FILENAME, content
            )
        })
    }

    pub fn char_count(&self) -> usize {
        self.char_count
    }

    pub fn is_truncated(&self) -> bool {
        self.truncated
    }

    pub fn is_loaded(&self) -> bool {
        self.content.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_load_nonexistent_file() {
        let dir = tempdir().unwrap();
        let (state, result) = MemoryState::load(dir.path());

        assert!(!state.is_loaded());
        assert!(matches!(result, LoadResult::NotFound));
    }

    #[test]
    fn test_load_existing_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AGENTS_MD_FILENAME);
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "# Test Memory\n\nThis is test content.").unwrap();

        let (state, result) = MemoryState::load(dir.path());

        assert!(state.is_loaded());
        assert!(!state.is_truncated());

        if let LoadResult::Loaded {
            char_count,
            truncated,
            ..
        } = result
        {
            assert!(char_count > 0);
            assert!(!truncated);
        } else {
            panic!("Expected LoadResult::Loaded");
        }
    }

    #[test]
    fn test_truncation() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AGENTS_MD_FILENAME);
        let mut file = File::create(&file_path).unwrap();

        // Write content exceeding MAX_CHAR_COUNT
        let large_content = "x".repeat(MAX_CHAR_COUNT + 1000);
        write!(file, "{}", large_content).unwrap();

        let (state, result) = MemoryState::load(dir.path());

        assert!(state.is_loaded());
        assert!(state.is_truncated());
        assert!(state.char_count() <= MAX_CHAR_COUNT + 100); // Allow for truncation message

        if let LoadResult::Loaded { truncated, .. } = result {
            assert!(truncated);
        } else {
            panic!("Expected LoadResult::Loaded");
        }
    }

    #[test]
    fn test_format_for_injection() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AGENTS_MD_FILENAME);
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Test content").unwrap();

        let (state, _) = MemoryState::load(dir.path());
        let formatted = state.format_for_injection().unwrap();

        assert!(formatted.contains("<project-memory source=\"AGENTS.md\">"));
        assert!(formatted.contains("Test content"));
        assert!(formatted.contains("</project-memory>"));
    }

    #[test]
    fn test_reload_if_changed() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join(AGENTS_MD_FILENAME);

        // Start with no file
        let (mut state, _) = MemoryState::load(dir.path());
        assert!(!state.is_loaded());

        // Create file
        let mut file = File::create(&file_path).unwrap();
        writeln!(file, "Initial content").unwrap();
        drop(file);

        // Should detect new file
        let result = state.reload_if_changed(dir.path());
        assert!(result.is_some());
        assert!(state.is_loaded());
    }
}

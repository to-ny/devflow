use std::path::Path;

use syntect::easy::HighlightLines;
use syntect::highlighting::{Style, ThemeSet};
use syntect::html::{styled_line_to_highlighted_html, IncludeBackground};
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub struct Highlighter {
    syntax_set: SyntaxSet,
    theme_set: ThemeSet,
}

impl Default for Highlighter {
    fn default() -> Self {
        Self::new()
    }
}

impl Highlighter {
    pub fn new() -> Self {
        Self {
            syntax_set: SyntaxSet::load_defaults_newlines(),
            theme_set: ThemeSet::load_defaults(),
        }
    }

    pub fn highlight_lines(&self, content: &str, file_path: &str) -> Vec<String> {
        let extension = Path::new(file_path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("");

        let mapped_extension = match extension {
            "ts" | "tsx" | "jsx" | "mjs" | "cjs" => "js",
            "yml" => "yaml",
            "md" => "markdown",
            "h" | "hpp" | "cc" => "cpp",
            ext => ext,
        };

        let syntax = self
            .syntax_set
            .find_syntax_by_extension(mapped_extension)
            .or_else(|| self.syntax_set.find_syntax_by_extension(extension))
            .or_else(|| self.syntax_set.find_syntax_by_first_line(content))
            .unwrap_or_else(|| self.syntax_set.find_syntax_plain_text());

        let theme = &self.theme_set.themes["base16-ocean.dark"];
        let mut highlighter = HighlightLines::new(syntax, theme);

        LinesWithEndings::from(content)
            .map(|line| {
                let ranges: Vec<(Style, &str)> = highlighter
                    .highlight_line(line, &self.syntax_set)
                    .unwrap_or_default();
                styled_line_to_highlighted_html(&ranges, IncludeBackground::No)
                    .unwrap_or_else(|_| html_escape(line))
            })
            .collect()
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_highlight_rust() {
        let highlighter = Highlighter::new();
        let code = "fn main() {\n    println!(\"Hello\");\n}";
        let highlighted = highlighter.highlight_lines(code, "test.rs");

        assert_eq!(highlighted.len(), 3);
        assert!(highlighted[0].contains("<span"));
    }

    #[test]
    fn test_highlight_tsx() {
        let highlighter = Highlighter::new();
        let code = "export function App() {\n  return <div>Hello</div>;\n}";
        let highlighted = highlighter.highlight_lines(code, "test.tsx");

        assert_eq!(highlighted.len(), 3);
        assert!(
            highlighted[0].contains("<span"),
            "TSX should be syntax highlighted via JS fallback"
        );
    }

    #[test]
    fn test_highlight_typescript() {
        let highlighter = Highlighter::new();
        let code = "const x: number = 42;";
        let highlighted = highlighter.highlight_lines(code, "test.ts");

        assert_eq!(highlighted.len(), 1);
        assert!(
            highlighted[0].contains("<span"),
            "TypeScript should be syntax highlighted via JS fallback"
        );
    }

    #[test]
    fn test_highlight_jsx() {
        let highlighter = Highlighter::new();
        let code = "const element = <h1>Hello</h1>;";
        let highlighted = highlighter.highlight_lines(code, "test.jsx");

        assert_eq!(highlighted.len(), 1);
        assert!(
            highlighted[0].contains("<span"),
            "JSX should be syntax highlighted via JS fallback"
        );
    }

    #[test]
    fn test_highlight_unknown_extension() {
        let highlighter = Highlighter::new();
        let code = "some plain text";
        let highlighted = highlighter.highlight_lines(code, "file.unknownext");

        assert_eq!(highlighted.len(), 1);
    }

    #[test]
    fn test_highlight_empty() {
        let highlighter = Highlighter::new();
        let highlighted = highlighter.highlight_lines("", "test.rs");

        assert!(highlighted.is_empty());
    }

    #[test]
    fn test_available_syntaxes() {
        let ss = SyntaxSet::load_defaults_newlines();

        println!("\nAvailable JS/TS syntaxes:");
        for syntax in ss.syntaxes() {
            let name_lower = syntax.name.to_lowercase();
            if name_lower.contains("typescript")
                || name_lower.contains("javascript")
                || name_lower.contains("jsx")
                || name_lower.contains("tsx")
            {
                println!(
                    "  {} - extensions: {:?}",
                    syntax.name, syntax.file_extensions
                );
            }
        }

        println!("\nDirect extension lookup:");
        println!(
            "  tsx: {:?}",
            ss.find_syntax_by_extension("tsx").map(|s| &s.name)
        );
        println!(
            "  ts: {:?}",
            ss.find_syntax_by_extension("ts").map(|s| &s.name)
        );
        println!(
            "  jsx: {:?}",
            ss.find_syntax_by_extension("jsx").map(|s| &s.name)
        );
        println!(
            "  js: {:?}",
            ss.find_syntax_by_extension("js").map(|s| &s.name)
        );
    }
}

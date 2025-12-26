Search file contents using regular expressions.

Use when:
- Finding where a function/variable is used
- Searching for patterns across codebase
- Locating specific text in files

Input:
- `pattern`: Regular expression to search for
- `path`: Directory to search in (optional, defaults to project root)
- `include`: File pattern filter like "*.rs" (optional)

Pattern syntax: Standard regex (Rust regex crate).
- `foo.*bar` - foo followed by bar
- `fn\s+\w+` - function definitions
- `TODO|FIXME` - either word

Returns matches in format: `file:line:content`

Use glob to find files by name; use grep to find files by content.

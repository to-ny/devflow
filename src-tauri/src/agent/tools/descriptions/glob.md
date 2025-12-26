Find files matching a glob pattern.

Use when:
- Finding files by name or extension
- Locating all files of a certain type
- Discovering project structure

Pattern syntax:
- `*` matches any characters except path separator
- `**` matches any characters including path separators (recursive)
- `?` matches single character
- `[abc]` matches any character in brackets

Examples:
- `**/*.rs` - all Rust files
- `src/**/*.ts` - TypeScript files in src
- `*.{js,ts}` - JS or TS files in root

Input:
- `pattern`: Glob pattern to match
- `path`: Base directory (optional, defaults to project root)

Returns matching file paths sorted by modification time (newest first).

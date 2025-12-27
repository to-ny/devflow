Fast file pattern matching tool that works with any codebase size.

## Usage

- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time (newest first)
- Use this when you need to find files by name patterns

## Parameters

- `pattern`: Glob pattern to match (required)
- `path`: Base directory (optional, defaults to project root)

## Pattern Syntax

- `*` matches any characters except path separator
- `**` matches any characters including path separators (recursive)
- `?` matches single character
- `[abc]` matches any character in brackets
- `{js,ts}` matches any of the comma-separated patterns

## Examples

| Pattern | Description |
|---------|-------------|
| `**/*.rs` | All Rust files in project |
| `src/**/*.ts` | TypeScript files in src directory |
| `*.{js,ts}` | JS or TS files in root |
| `src/**/test_*.py` | Python test files in src |
| `**/config.{json,yaml,toml}` | Config files anywhere |

## When to Use

- Finding files by name or extension
- Locating all files of a certain type
- Discovering project structure
- Searching for specific file patterns

## When NOT to Use

- Searching for content within files (use grep instead)
- Listing a single directory (use list_directory instead)
- Open-ended exploration requiring multiple rounds (use dispatch_agent instead)

## Best Practices

- Use multiple glob calls in parallel when searching for different patterns
- Combine with read_file to examine matching files
- For content-based search, use grep instead

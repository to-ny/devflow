Fast content search tool that searches file contents using regular expressions.

## Usage

- Searches file contents using regex patterns
- Supports full regex syntax (e.g., "log.*Error", "function\\s+\\w+")
- Filter files by pattern with the include parameter
- Returns file paths with matches sorted by modification time

## Parameters

- `pattern`: Regular expression to search for (required)
- `path`: Directory to search in (optional, defaults to project root)
- `include`: File pattern filter like "*.rs" or "*.{ts,tsx}" (optional)

## Pattern Syntax (Rust Regex)

| Pattern | Description |
|---------|-------------|
| `foo.*bar` | foo followed by bar |
| `fn\s+\w+` | Function definitions |
| `TODO\|FIXME` | Either word |
| `import.*from` | ES6 imports |
| `class\s+\w+` | Class definitions |

## Return Format

Returns matches in format: `file:line:content`

## When to Use

- Finding where a function/variable is used
- Searching for patterns across codebase
- Locating specific text in files
- Finding TODO/FIXME comments

## When NOT to Use

- Finding files by name (use glob instead)
- If you need to count matches within files (use bash with `rg` directly)
- Open-ended exploration (use dispatch_agent instead)

## Best Practices

- Use include filter to narrow search scope for better performance
- Use multiple grep calls in parallel for different patterns
- For class/function definitions, prefer glob with naming conventions first
- If you still need grep functionality in bash, use ripgrep (`rg`) instead of grep

## Example

```json
{
  "pattern": "async fn\\s+\\w+",
  "path": "src",
  "include": "*.rs"
}
```

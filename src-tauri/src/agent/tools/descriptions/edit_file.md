Performs exact string replacements in files.

## Prerequisites

- You MUST use read_file at least once before editing
- This tool will error if you attempt an edit without reading the file first

## Usage

- `path`: Relative path from project root
- `old_text`: Exact text to find (must match including whitespace)
- `new_text`: Replacement text
- `replace_all`: Replace all occurrences (optional, default false)

## Critical Requirements

### Preserve Indentation
- When editing text, preserve the exact indentation (tabs/spaces) as it appears in the file
- Never include line number prefixes in old_text or new_text

### Ensure Uniqueness
- The edit will FAIL if old_text is not unique in the file
- Provide a larger string with more surrounding context to make it unique
- Or use replace_all to change every instance

### Use replace_all for Renaming
- Use replace_all when replacing/renaming strings across the file
- This is useful for renaming variables, functions, or classes

## Best Practices

- ALWAYS prefer editing existing files over creating new ones
- Only use emojis if the user explicitly requests it
- Make targeted, minimal changes to accomplish the task
- For multiple related changes to one file, use multi_edit instead

## When to Use

- Making targeted modifications to existing files
- Updating specific functions, imports, or configurations
- Small changes where you know the exact text to replace
- Renaming variables or functions (with replace_all)

## Examples

Simple replacement:
```json
{
  "path": "src/config.ts",
  "old_text": "const API_URL = \"http://localhost\"",
  "new_text": "const API_URL = \"https://api.example.com\""
}
```

Rename across file:
```json
{
  "path": "src/utils.ts",
  "old_text": "getUserName",
  "new_text": "getUsername",
  "replace_all": true
}
```

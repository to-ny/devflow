Replace text in a file while preserving the rest of the content.

Use when:
- Making targeted modifications to existing files
- Updating specific functions, imports, or configurations
- Small changes where you know the exact text to replace

Input:
- `path`: Relative path from project root
- `old_text`: Exact text to find (must match including whitespace)
- `new_text`: Replacement text
- `replace_all`: Replace all occurrences (optional, default false)

For multiple related changes to one file, use multi_edit instead.

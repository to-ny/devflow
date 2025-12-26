Apply multiple text replacements to a file atomically.

Use when:
- Making several related changes to one file
- Refactoring (renaming variables, updating imports)
- Changes that should succeed or fail together

Input:
- `path`: Relative path from project root
- `edits`: Array of { old_text, new_text } pairs

Edits are applied sequentially. If any edit fails, all changes are rolled back.

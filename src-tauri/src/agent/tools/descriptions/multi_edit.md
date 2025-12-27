Apply multiple text replacements to a single file in one atomic operation.

Prefer this tool over edit_file when you need to make multiple edits to the same file.

## Prerequisites

- You MUST use read_file first to understand the file's contents
- Verify the file path is correct before editing

## Parameters

- `path`: Relative path from project root (required)
- `edits`: Array of edit operations (required), where each edit contains:
  - `old_text`: The text to replace (must match exactly, including whitespace)
  - `new_text`: The replacement text

## How It Works

- All edits are applied in sequence, in the order provided
- Each edit operates on the result of the previous edit
- All edits must be valid for the operation to succeed
- If any edit fails, none will be applied (atomic operation)

## Critical Requirements

1. All edits follow the same requirements as edit_file
2. The edits are atomic - either all succeed or none are applied
3. Plan your edits carefully to avoid conflicts between sequential operations
4. Since edits are applied in sequence, ensure earlier edits don't affect text that later edits need to find

## When to Use

- Making several related changes to one file
- Refactoring (renaming variables, updating imports)
- Changes that should succeed or fail together
- Multiple find-and-replace operations in the same file

## Warnings

- The tool will fail if any old_text doesn't match the file contents exactly
- The tool will fail if old_text and new_text are the same
- Only use emojis if the user explicitly requests it

## Example

```json
{
  "path": "src/config.ts",
  "edits": [
    {
      "old_text": "const API_URL = \"http://localhost:3000\"",
      "new_text": "const API_URL = \"https://api.example.com\""
    },
    {
      "old_text": "const TIMEOUT = 5000",
      "new_text": "const TIMEOUT = 10000"
    }
  ]
}
```

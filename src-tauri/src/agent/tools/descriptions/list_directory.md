Lists files and directories at a given path.

## Usage

- The path parameter should be relative to project root
- Use "." for the project root directory
- Returns entries with their types (file/directory)
- Output is sorted alphabetically

## Parameters

- `path`: Relative path to directory (required)

## When to Use

- Exploring project structure
- Checking what files exist before reading or creating
- Understanding directory layout
- Verifying parent directories exist before creating files

## When NOT to Use

- Finding files by pattern across directories (use glob instead)
- Searching for files by content (use grep instead)

## Best Practices

- Use this before running mkdir to verify parent directories exist
- Prefer glob when you know the pattern you're looking for
- Prefer grep when you know the content you're looking for

## Example

```json
{"path": "src/components"}
```

Returns:
```
Button.tsx (file)
Modal.tsx (file)
forms/ (dir)
utils/ (dir)
```

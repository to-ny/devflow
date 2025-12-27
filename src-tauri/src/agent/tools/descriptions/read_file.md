Reads a file from the filesystem. You can access any file directly by using this tool.

## Usage

- The path parameter should be relative to the project root
- By default, reads up to 2000 lines starting from the beginning
- You can optionally specify offset and limit for large files
- Any lines longer than 2000 characters will be truncated
- Results are returned with line numbers starting at 1

## Parameters

- `path`: Relative path from project root (required)
- `offset`: Starting line number (optional)
- `limit`: Number of lines to read (optional)

## When to Use

- Reading source code files
- Understanding file contents before editing
- Checking configuration files
- Reviewing existing implementations

## Important Notes

- Assume this tool can read all files in the project
- If a file doesn't exist, an error will be returned
- For Jupyter notebooks (.ipynb files), use notebook_read instead
- It's always better to speculatively read multiple files in parallel that are potentially useful
- If you read a file that exists but has empty contents, you will receive a warning
- Always read a file before using edit_file to ensure exact text matching

## Examples

Read entire file:
```json
{"path": "src/main.rs"}
```

Read specific range:
```json
{"path": "src/lib.rs", "offset": 100, "limit": 50}
```

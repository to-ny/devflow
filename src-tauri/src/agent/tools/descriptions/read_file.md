Read the contents of a file.

Use when:
- Examining file contents before editing
- Understanding existing code
- Checking configuration files

Always read a file before using edit_file to ensure exact text matching.

Input:
- `path`: Relative path from project root
- `offset`: Starting line number (optional)
- `limit`: Number of lines to read (optional)

Returns the file contents. Binary files will return garbled content. Very large files may be truncated.

Errors if the file doesn't exist or path is outside project directory.

Create a new file or completely replace an existing file's contents.

Use when:
- Creating new files
- Replacing entire file contents
- Writing generated code or configuration

Avoid when:
- Making partial edits (use edit_file or multi_edit)
- File exists and you only need to change part of it

Parent directories are created automatically if they don't exist. Overwrites without confirmation - read first if unsure about existing content.

Errors if path is outside project directory.

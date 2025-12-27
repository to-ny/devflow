Writes a file to the filesystem. This will create or overwrite the file at the specified path.

## Usage

- The path parameter should be relative to the project root
- The content parameter is the complete file content to write
- This tool will overwrite existing files completely
- Parent directories are created automatically if they don't exist

## Prerequisites

- If this is an existing file, you MUST use read_file first to read its contents
- This tool will fail if you did not read an existing file first

## Important Rules

- ALWAYS prefer editing existing files using edit_file instead of write_file
- NEVER write new files unless explicitly required by the task
- NEVER proactively create documentation files (*.md) or README files
- Only create documentation files if explicitly requested by the user
- Only use emojis if the user explicitly requests it

## When to Use

- Creating new source files that don't exist
- Generating configuration files
- Creating new test files
- Only when edit_file is not appropriate

## When NOT to Use

- Modifying existing files (use edit_file instead)
- Making targeted changes to code (use edit_file instead)
- Creating documentation without being asked

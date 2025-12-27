Executes a bash command in a persistent shell session with optional timeout.

## Before Executing Commands

### Directory Verification
- If the command will create new directories or files, first use list_directory to verify the parent directory exists and is the correct location
- For example, before running "mkdir foo/bar", first check that "foo" exists

### Command Execution
- Always quote file paths that contain spaces with double quotes
- Examples of proper quoting:
  - cd "/Users/name/My Documents" (correct)
  - cd /Users/name/My Documents (incorrect - will fail)
  - python "/path/with spaces/script.py" (correct)

## Usage Notes

- The command argument is required
- Optional timeout in seconds (default: 30, max: 600)
- Output is truncated if it exceeds 30,000 characters
- Commands run with the project root as working directory

## IMPORTANT: Avoid These Commands

You MUST avoid using these commands and use dedicated tools instead:
- `find` - use glob instead
- `grep` - use grep tool instead
- `cat`, `head`, `tail` - use read_file instead
- `ls` - use list_directory instead

If you still need grep functionality, use ripgrep (`rg`) which is more efficient.

## Command Chaining

- Use `;` or `&&` to chain multiple commands
- DO NOT use newlines to separate commands (newlines are ok in quoted strings)
- Prefer `&&` when commands depend on each other
- Use `;` when you want all commands to run regardless of success

## Best Practices

- Use absolute paths to maintain working directory throughout the session
- Avoid `cd` unless the user explicitly requests it

### Good Example
```
pytest /foo/bar/tests
```

### Bad Example
```
cd /foo/bar && pytest tests
```

## Common Operations

- Run tests: `pytest /path/to/tests` or `npm test`
- Build: `npm run build` or `cargo build`
- Git operations: `git status`, `git diff`, `git log`
- View PR comments: `gh api repos/owner/repo/pulls/123/comments`

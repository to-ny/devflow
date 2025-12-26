Execute shell commands in the project directory.

Use when:
- Running CLI tools (git, npm, cargo, make, etc.)
- Build and test commands
- Any terminal operation not covered by other tools

Avoid when:
- Reading files (use read_file)
- Searching file contents (use grep)
- Finding files (use glob)
- Listing directories (use list_directory)

The command runs with the project root as working directory. Both stdout and stderr are captured. Commands that exceed the timeout will be terminated.

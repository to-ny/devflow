Fast file pattern matching tool that works with any codebase size.

**IMPORTANT**: ALWAYS use this tool to find files by name patterns. NEVER use `find` or `ls` via bash for file discovery.

- Supports glob patterns like "**/*.js" or "src/**/*.ts"
- Returns matching file paths sorted by modification time
- Use this tool when you need to find files by name patterns
- When you are doing an open ended search that may require multiple rounds of globbing and grepping, use the dispatch_agent tool instead
- You can call multiple tools in a single response. It is always better to speculatively perform multiple searches in parallel if they are potentially useful.

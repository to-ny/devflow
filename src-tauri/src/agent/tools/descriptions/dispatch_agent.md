Launch a sub-agent to handle complex tasks autonomously.

The sub-agent has access to file and search tools (read_file, glob, grep, list_directory, web_fetch, search_web) and works independently, returning results when complete.

## Parameters

- `task`: Detailed task instructions for the sub-agent (required)
- `tools`: Optional list of tool names to allow (default: read-only tools)

## When to Use

- Task requires extensive searching or exploration
- Searching for a keyword like "config" or "logger"
- Questions like "which file does X?"
- Multiple uncertain steps needed
- Work can be parallelized
- Open-ended exploration where exact steps aren't known

## When NOT to Use

- Reading a specific known file (use read_file directly)
- Searching for a specific class like "class Foo" (use glob)
- Searching within 2-3 specific files (use read_file)
- Simple, well-defined operations
- Tasks requiring user interaction
- Writing code (parent agent should do this)

## Usage Guidelines

1. **Launch concurrently** - Launch multiple agents in parallel when possible
2. **Results not visible to user** - You must summarize results for the user
3. **Stateless** - Each agent invocation is independent; include all context in the task
4. **Trust outputs** - Agent results should generally be trusted
5. **Be specific** - Clearly tell the agent:
   - Whether to write code or just research
   - Exactly what information to return
   - All relevant context for the task

## Example

```json
{
  "task": "Search the codebase to find all files that handle user authentication. Look for login, logout, session management, and token handling. Return a list of file paths with brief descriptions of what each file does."
}
```

```json
{
  "task": "Find where the API rate limiting is implemented. Search for rate limit, throttle, and request limit patterns. Return the file paths and relevant code snippets."
}
```

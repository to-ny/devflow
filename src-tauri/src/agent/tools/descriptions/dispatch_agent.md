Launch a specialized agent to handle complex, multi-step tasks autonomously.

The dispatch_agent tool launches sub-agents with specific capabilities. Each agent type has its own system prompt and tool access tailored to its purpose.

## Agent Types

| Type | Purpose | Tool Access |
|------|---------|-------------|
| `explore` | Fast codebase exploration (default) | read_file, glob, grep, list_directory, bash, web_fetch, search_web |
| `plan` | Design implementation strategies | read_file, glob, grep, list_directory, bash, dispatch_agent |
| `pr-review` | Review pull requests | read_file, glob, grep, bash |
| `pr-comments` | Fetch and analyze PR comments | bash, web_fetch |
| `security-review` | Security-focused code review | read_file, glob, grep |
| `summarize` | Summarize conversations | None (text only) |
| `bash-summarize` | Summarize command output | None (text only) |
| `session-title` | Generate titles and branch names | None (text only) |

## When to Use

Use dispatch_agent for:
- Open-ended codebase exploration that may require multiple search rounds
- Specialized tasks like PR review or security analysis
- Tasks that benefit from focused, autonomous execution

## When NOT to Use

- If you want to read a specific file, use read_file directly
- If searching for a specific class like "class Foo", use glob directly
- If searching within 2-3 known files, use read_file directly

## Usage Notes

- Provide clear, detailed task descriptions
- Launch multiple agents in parallel when tasks are independent
- The agent's output is returned to you, not the user - summarize results for the user
- Trust agent outputs and provide comprehensive prompts
- Specify whether the agent should research or write code

## Examples

<example>
user: "Where is the authentication logic?"
assistant: I'll use dispatch_agent to search the codebase.
*Uses dispatch_agent with task: "Find all files that handle user authentication, including login, logout, and session management." and agent_type: "explore"*
</example>

<example>
user: "Review PR #123"
assistant: Let me review that pull request.
*Uses dispatch_agent with task: "Review PR #123 for code quality, bugs, and security issues." and agent_type: "pr-review"*
</example>

<example>
user: "Check the security of the new API endpoints"
assistant: I'll run a security review.
*Uses dispatch_agent with task: "Analyze the API endpoint code for security vulnerabilities including injection, auth bypasses, and data exposure." and agent_type: "security-review"*
</example>

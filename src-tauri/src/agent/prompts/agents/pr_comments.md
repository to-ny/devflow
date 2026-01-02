You are tasked with fetching and presenting GitHub pull request comments.

## Process

1. Get PR metadata: `gh pr view <number>`
2. Fetch PR-level comments: `gh api repos/{owner}/{repo}/issues/{number}/comments`
3. Fetch code review comments: `gh api repos/{owner}/{repo}/pulls/{number}/comments`

## For code review comments, extract:
- `body`: The comment text
- `diff_hunk`: The code context
- `path`: The file path
- `line`: The line number
- `user.login`: The commenter

## Output Format

Present comments in a clear, threaded structure:

```
## File: path/to/file.rs

### Line 42 (@username)
> Code context from diff_hunk

Comment text here

#### Reply (@another_user)
Reply text here
```

## Guidelines

- Show only the actual comments, no additional text
- Preserve threading and nesting relationships
- Include code context for review comments
- Use `jq` for JSON parsing when needed
- If no comments exist, respond with "No comments found."

Fetch and display the PR comments now.

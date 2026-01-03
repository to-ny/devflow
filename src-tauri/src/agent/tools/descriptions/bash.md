Executes a bash command in a persistent shell session.

**CRITICAL**: ONLY for terminal operations. Use dedicated tools instead:
- `read_file` for reading (NOT cat/head/tail)
- `grep` for searching (NOT grep/rg command)
- `glob` for finding files (NOT find/ls)
- `edit_file` for editing (NOT sed/awk)
- `write_file` for writing (NOT echo/heredoc)

**ALLOWED**: git, npm/cargo/pip, build tools, docker, system commands.

Usage notes:
- Required: command. Optional: timeout (default 30s, max 600s).
- Quote paths with spaces: `cd "/path with spaces"`
- Output truncated at 30000 chars.
- Use `run_in_background` for long-running commands.
- Multiple commands: use `&&` for sequential, parallel tool calls for independent.
- Use absolute paths; avoid `cd`.

# Git Commits

Only commit when explicitly requested. Git Safety:
- NEVER: update git config, force push, skip hooks, amend pushed commits
- Use --amend ONLY if: user requested OR hook auto-modified files, commit is yours, not pushed
- If commit fails, create NEW commit (never amend failed commits)

Steps:
1. Run in parallel: `git status`, `git diff`, `git log` (recent messages)
2. Draft commit message (focus on "why", 1-2 sentences)
3. Stage files, commit with HEREDOC format, verify with `git status`

```
git commit -m "$(cat <<'EOF'
Commit message here.

Co-Authored-By: AI Assistant <noreply@example.com>
EOF
)"
```

Never: push unless asked, use -i flags, commit secrets, create empty commits.

# Pull Requests

Use `gh` for GitHub tasks. Steps:
1. Run parallel: `git status`, `git diff`, `git log`, `git diff [base]...HEAD`
2. Draft PR summary covering ALL commits
3. Push if needed, create PR with HEREDOC body

```
gh pr create --title "Title" --body "$(cat <<'EOF'
## Summary
<bullets>

## Test plan
<checklist>
EOF
)"
```

Return the PR URL when done.

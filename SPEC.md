# Devflow Specification

Desktop application for AI-assisted iterative code development with integrated diff review.

## Workflow

1. User opens project directory
2. User sends prompt to agent via chat input
3. Agent executes (file ops, shell commands), output streams in real-time
4. Agent signals completion → notification triggered
5. Diff view auto-refreshes with unstaged changes
6. User reviews diff, adds comments (global and/or line-specific)
7. User sends comments → formatted via template → sent to agent
8. Agent revises → repeat from step 5 until satisfied
9. User clicks "Commit" → `git add --all` → commit template sent to agent

## Architecture

```
Frontend (React + Vite + TypeScript):
  WelcomeScreen, ChatPanel, DiffView, FileTree, CommentEditor, CommitModal

Backend (Tauri + Rust):
  AgentOrchestrator, ProviderAdapter (trait) → AnthropicAdapter, ToolExecutor (trait) → LocalExecutor,
  GitService, ConfigService, TemplateService

Communication: Tauri commands (invoke) and events (emit)
```

## Features

### Application Shell

- Launch: check last_project in app config → if valid, open; else show WelcomeScreen
- WelcomeScreen: "Open Project" button → native folder picker → load project
- Native menu bar: File → Open Project, Close Project, Quit
- Window title: "Devflow - {project_name}" or "Devflow" if no project

### Layout

Three-column layout (all resizable):
- Left: FileTree (changed files list, click to select)
- Center: DiffView (unified diff, syntax highlighted, comment overlay)
- Right: ChatPanel (prompt input, message history, streaming output)

### Chat Panel

- Prompt input with Send button
- Streaming message history (user prompts, agent responses, tool executions)
- Prompt history dropdown (last 50, in-memory)
- Pre/post prompt injection (from config, invisible to user)
- "Send Comments" button → renders template, sends to agent, clears comments
- "Commit" button → opens CommitModal

### Diff View

- Unified diff format with syntax highlighting
- Line numbers (old and new)
- Click line or drag range → opens CommentEditor
- Visual indicators for commented lines
- Auto-refresh on file system changes

### File Tree

- Lists files with unstaged changes (added/modified/deleted icons)
- Click to select → DiffView updates
- Badge showing comment count per file

### Comments

- Global comments (entire changeset) and line-specific comments
- Stored in memory until "Send Comments" clicked
- Template variables: `{{comments}}` (array with file, lines, selected_code, text), `{{global_comment}}`

### Commit Flow

CommitModal: text input for instructions → executes `git add --all` → renders commit template → sends to agent

### Notifications

Events: agent complete, agent error
Actions: window flash/highlight, optional sound (configurable per event)

### Input Behavior

Prompt input submits on Enter (Shift+Enter for newline).

## Configuration

### App Config

Location: Tauri `app_data_dir` / `app.toml`

Schema: `[state]` with `last_project` (string path)

### Project Config

Location: `<project>/.devflow/config.toml`

Schema:
- `[agent]`: provider (string), model (string), api_key_env (string, env var name), max_tokens (int)
- `[prompts]`: pre (string), post (string)
- `[execution]`: timeout_secs (int)
- `[notifications]`: on_complete, on_error (arrays, values: "sound", "window")

## Templates

Location: `~/.config/devflow/templates/`

Variables available:
- review-comments.txt: `{{comments}}` (array: file, lines.start, lines.end, language, selected_code, text), `{{global_comment}}`
- commit.txt: `{{instructions}}`, `{{files}}` (array of paths)

## MVP Scope

Included: Single project, Anthropic only, LocalExecutor, unified diff with syntax highlighting, comments, pre/post prompts, in-memory prompt history, notifications, commit flow

Excluded: Multi-project, other providers, ContainerExecutor, session persistence, prompt history persistence, permission system

## Implementation Notes

Key crates: `tauri`, `tokio`, `reqwest`, `syntect`, `handlebars`, `serde`, `toml`, `toml_edit`, `notify`, `glob`

Implementation order:
1. Tauri + React + Vite scaffolding
2. Config loading
3. Git service (diff via Git CLI)
4. UI shell with layout
5. DiffView + FileTree
6. AnthropicAdapter + ChatPanel streaming
7. ToolExecutor (LocalExecutor)
8. AgentOrchestrator (tool loop)
9. Template rendering
10. Notifications
11. Commit flow

Gotchas:
- Anthropic API uses server-sent events for streaming
- Git CLI: use `git status --porcelain -uall` for changed files, `git diff -- <file>` for diffs
- WSL paths: route git commands through `wsl.exe -d <distro> git -C <path>` for proper .gitignore handling
- Debounce file watcher events

Tool definitions for Anthropic API:
- bash: `{ command: string }` — execute shell command
- read_file: `{ path: string }` — read file contents
- write_file: `{ path: string, content: string }` — create/overwrite file
- edit_file: `{ path: string, old_text: string, new_text: string }` — replace text in file
- list_directory: `{ path: string }` — list directory contents

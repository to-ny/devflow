# Devflow

> **Note:** This project is no longer maintained. The cost of LLM API consumption made it impractical compared to using provider tools directly.

Desktop app for AI-assisted iterative code development with integrated diff review.

## Features

- Chat with AI agents (Anthropic Claude, Google Gemini)
- Real-time streaming responses with tool execution
- Integrated diff viewer with line-level commenting
- Review workflow: comment on changes, send feedback, iterate
- Commit flow with AI-generated messages

## Tech Stack

- **Frontend**: React, TypeScript, Vite, TailwindCSS
- **Backend**: Rust, Tauri

## Development

```bash
npm install      # Install dependencies
make dev         # Run dev server
make build       # Production build
make test        # Run tests
make lint        # Lint code
make fmt         # Format code
```

## Configuration

Create `.devflow/config.toml` in your project:

```toml
[agent]
provider = "anthropic"  # or "gemini"
model = "claude-sonnet-4-20250514"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 8192

[execution]
timeout_secs = 120
max_tool_iterations = 50
```

## License

MIT + Commons Clause - See [LICENSE](LICENSE)

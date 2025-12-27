# Devflow

Desktop app for AI-assisted iterative code development with integrated Git diff review.

## Stack

- Frontend: React + Vite + TypeScript
- Backend: Tauri + Rust
- Providers: Anthropic, Gemini

## Structure

- `src/` - React frontend (pages, components, context)
- `src-tauri/src/` - Rust backend (agent, config, git, tools)
- `src/types/generated/` - TypeScript types from Rust (ts-rs)

## Key Patterns

- Tauri commands for invoke, events for streaming
- Provider trait pattern: `ProviderAdapter` → `AnthropicAdapter` | `GeminiAdapter`
- Tool executor pattern: `ToolExecutor` trait → `LocalExecutor`
- Config: `.devflow/config.toml` per project

## Commands

- `make test` - run all tests
- `make fmt` - format code
- `make lint` - lint code
- `make build-windows` - build Windows executable
- `make build-windows-dev` - build Windows executable with devtools - ONLY for investigation with logging

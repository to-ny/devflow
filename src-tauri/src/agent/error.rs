use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error("Missing API key: environment variable '{0}' not set")]
    MissingApiKey(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("API error: {0}")]
    ApiError(String),

    #[error("Config error: {0}")]
    ConfigError(String),

    #[error("Unsupported provider: '{0}'. Supported providers: anthropic")]
    UnsupportedProvider(String),

    #[error("Tool execution error: {0}")]
    ToolExecutionError(String),

    #[error("Tool execution timed out")]
    ToolTimeout,

    #[error("Invalid tool input: {0}")]
    InvalidToolInput(String),

    #[error("Unknown tool: {0}")]
    UnknownTool(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

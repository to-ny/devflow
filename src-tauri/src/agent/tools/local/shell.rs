use std::path::PathBuf;
use std::time::Duration;

use tokio::process::Command;
use tokio::time::timeout;

use super::context::ExecutionContext;
use crate::agent::error::AgentError;
use crate::agent::tools::types::BashInput;

#[cfg(windows)]
use crate::git::wsl::WslPath;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct ShellExecutor {
    working_dir: PathBuf,
    default_timeout: Duration,
    #[cfg(windows)]
    wsl_path: Option<WslPath>,
}

impl ShellExecutor {
    #[cfg(windows)]
    pub fn new(working_dir: PathBuf, default_timeout: Duration, wsl_path: Option<WslPath>) -> Self {
        Self {
            working_dir,
            default_timeout,
            wsl_path,
        }
    }

    #[cfg(not(windows))]
    pub fn new(working_dir: PathBuf, default_timeout: Duration) -> Self {
        Self {
            working_dir,
            default_timeout,
        }
    }

    async fn run_command(&self, command: &str) -> std::io::Result<std::process::Output> {
        #[cfg(target_os = "windows")]
        {
            if let Some(ref wsl) = self.wsl_path {
                Command::new("wsl.exe")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["-d", &wsl.distro, "sh", "-c"])
                    .arg(format!("cd '{}' && {}", wsl.linux_path, command))
                    .output()
                    .await
            } else {
                Command::new("cmd")
                    .creation_flags(CREATE_NO_WINDOW)
                    .args(["/C", command])
                    .current_dir(&self.working_dir)
                    .output()
                    .await
            }
        }

        #[cfg(not(target_os = "windows"))]
        {
            Command::new("sh")
                .arg("-c")
                .arg(command)
                .current_dir(&self.working_dir)
                .output()
                .await
        }
    }

    pub async fn execute(&self, input: serde_json::Value) -> Result<String, AgentError> {
        let input: BashInput = serde_json::from_value(input)
            .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

        let cmd_timeout = input
            .timeout
            .map(Duration::from_secs)
            .unwrap_or(self.default_timeout);

        let result = timeout(cmd_timeout, self.run_command(&input.command)).await;

        match result {
            Ok(Ok(output)) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);

                let combined = if stderr.is_empty() {
                    stdout.to_string()
                } else if stdout.is_empty() {
                    stderr.to_string()
                } else {
                    format!("{}\n{}", stdout, stderr)
                };

                let combined = ExecutionContext::truncate_output(combined);

                if output.status.success() {
                    Ok(combined)
                } else {
                    Err(AgentError::ToolExecutionError(format!(
                        "Command failed with exit code {}: {}",
                        output.status.code().unwrap_or(-1),
                        combined
                    )))
                }
            }
            Ok(Err(e)) => Err(AgentError::ToolExecutionError(format!(
                "Failed to execute command: {}",
                e
            ))),
            Err(_) => Err(AgentError::ToolTimeout),
        }
    }
}

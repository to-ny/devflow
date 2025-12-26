use log::debug;
use tokio::fs;

use super::context::ExecutionContext;
use crate::agent::error::AgentError;
use crate::agent::tools::types::{NotebookEditInput, NotebookReadInput};

pub async fn read(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: NotebookReadInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;
    debug!("Reading notebook: {}", path.display());

    let content = ctx
        .with_timeout("read notebook", fs::read_to_string(&path))
        .await?;

    let notebook: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AgentError::ToolExecutionError(format!("Invalid notebook JSON: {}", e)))?;

    let cells = notebook
        .get("cells")
        .and_then(|c| c.as_array())
        .ok_or_else(|| AgentError::ToolExecutionError("Notebook has no cells array".to_string()))?;

    let output: Vec<String> = cells
        .iter()
        .enumerate()
        .map(|(i, cell)| {
            let cell_type = cell
                .get("cell_type")
                .and_then(|t| t.as_str())
                .unwrap_or("unknown");

            let source = cell
                .get("source")
                .and_then(|s| {
                    if let Some(arr) = s.as_array() {
                        Some(
                            arr.iter()
                                .filter_map(|v| v.as_str())
                                .collect::<Vec<_>>()
                                .join(""),
                        )
                    } else {
                        s.as_str().map(|s| s.to_string())
                    }
                })
                .unwrap_or_default();

            format!("--- Cell {} ({}) ---\n{}", i, cell_type, source)
        })
        .collect();

    Ok(output.join("\n\n"))
}

pub async fn edit(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: NotebookEditInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;
    debug!(
        "Editing notebook: {} (cell {})",
        path.display(),
        input.cell_number
    );

    let content = ctx
        .with_timeout("read notebook", fs::read_to_string(&path))
        .await?;

    let mut notebook: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| AgentError::ToolExecutionError(format!("Invalid notebook JSON: {}", e)))?;

    let cells = notebook
        .get_mut("cells")
        .and_then(|c| c.as_array_mut())
        .ok_or_else(|| AgentError::ToolExecutionError("Notebook has no cells array".to_string()))?;

    let edit_mode = input.edit_mode.as_deref().unwrap_or("replace");
    let cell_idx = input.cell_number as usize;

    match edit_mode {
        "replace" => {
            if cell_idx >= cells.len() {
                return Err(AgentError::ToolExecutionError(format!(
                    "Cell {} does not exist (notebook has {} cells)",
                    cell_idx,
                    cells.len()
                )));
            }
            let source_lines: Vec<serde_json::Value> = input
                .new_source
                .lines()
                .map(|l| serde_json::Value::String(format!("{}\n", l)))
                .collect();
            cells[cell_idx]["source"] = serde_json::Value::Array(source_lines);
        }
        "insert" => {
            let cell_type = input.cell_type.as_deref().unwrap_or("code");
            let source_lines: Vec<serde_json::Value> = input
                .new_source
                .lines()
                .map(|l| serde_json::Value::String(format!("{}\n", l)))
                .collect();
            let new_cell = serde_json::json!({
                "cell_type": cell_type,
                "source": source_lines,
                "metadata": {},
                "outputs": []
            });
            if cell_idx > cells.len() {
                cells.push(new_cell);
            } else {
                cells.insert(cell_idx, new_cell);
            }
        }
        "delete" => {
            if cell_idx >= cells.len() {
                return Err(AgentError::ToolExecutionError(format!(
                    "Cell {} does not exist (notebook has {} cells)",
                    cell_idx,
                    cells.len()
                )));
            }
            cells.remove(cell_idx);
        }
        _ => {
            return Err(AgentError::InvalidToolInput(format!(
                "Unknown edit_mode: {}",
                edit_mode
            )))
        }
    }

    let new_content = serde_json::to_string_pretty(&notebook)
        .map_err(|e| AgentError::ToolExecutionError(format!("Failed to serialize: {}", e)))?;

    ctx.with_timeout("write notebook", fs::write(&path, &new_content))
        .await?;

    Ok(format!(
        "Successfully {} cell {} in {}",
        edit_mode,
        cell_idx,
        path.display()
    ))
}

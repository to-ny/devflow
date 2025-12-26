use std::collections::BinaryHeap;
use std::path::PathBuf;
use std::time::SystemTime;

use glob::glob as glob_match;
use regex::Regex;
use tokio::fs;
use walkdir::WalkDir;

use super::context::ExecutionContext;
use crate::agent::error::AgentError;
use crate::agent::tools::types::{
    EditFileInput, GlobInput, GrepInput, ListDirectoryInput, MultiEditInput, ReadFileInput,
    WriteFileInput,
};

const MAX_GLOB_RESULTS: usize = 1000;
const MAX_GREP_RESULTS: usize = 100;
const BINARY_CHECK_SIZE: usize = 1024;

#[derive(Eq, PartialEq)]
struct GlobEntry {
    path: PathBuf,
    mtime: SystemTime,
}

impl Ord for GlobEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.mtime.cmp(&other.mtime)
    }
}

impl PartialOrd for GlobEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

pub async fn read_file(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: ReadFileInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;

    let content = ctx
        .with_timeout("read file", fs::read_to_string(&path))
        .await?;

    let lines: Vec<&str> = content.lines().collect();
    let offset = input.offset.unwrap_or(0) as usize;
    let limit = input.limit.map(|l| l as usize).unwrap_or(lines.len());

    let result: String = lines
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect::<Vec<_>>()
        .join("\n");

    Ok(result)
}

pub async fn write_file(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: WriteFileInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent).await?;
        }
    }

    ctx.with_timeout("write file", fs::write(&path, &input.content))
        .await?;
    Ok(format!("Successfully wrote to {}", path.display()))
}

pub async fn edit_file(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: EditFileInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;

    let content = ctx
        .with_timeout("read file", fs::read_to_string(&path))
        .await?;

    if !content.contains(&input.old_text) {
        return Err(AgentError::ToolExecutionError(
            "old_text not found in file".to_string(),
        ));
    }

    let new_content = if input.replace_all.unwrap_or(false) {
        content.replace(&input.old_text, &input.new_text)
    } else {
        content.replacen(&input.old_text, &input.new_text, 1)
    };

    ctx.with_timeout("write file", fs::write(&path, &new_content))
        .await?;
    Ok(format!("Successfully edited {}", path.display()))
}

pub async fn multi_edit(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: MultiEditInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;

    let content = ctx
        .with_timeout("read file", fs::read_to_string(&path))
        .await?;

    let mut result = content;
    for (i, edit) in input.edits.iter().enumerate() {
        if !result.contains(&edit.old_text) {
            return Err(AgentError::ToolExecutionError(format!(
                "Edit {} failed: old_text not found in file",
                i + 1
            )));
        }
        result = result.replacen(&edit.old_text, &edit.new_text, 1);
    }

    ctx.with_timeout("write file", fs::write(&path, &result))
        .await?;
    Ok(format!(
        "Successfully applied {} edits to {}",
        input.edits.len(),
        path.display()
    ))
}

pub async fn list_directory(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: ListDirectoryInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let path = ctx.resolve_path(&input.path)?;

    let mut read_dir = ctx
        .with_timeout("read directory", fs::read_dir(&path))
        .await?;
    let mut entries = Vec::new();

    while let Ok(Some(entry)) = read_dir.next_entry().await {
        let name = entry.file_name().to_string_lossy().to_string();
        let file_type = match entry.file_type().await {
            Ok(ft) if ft.is_dir() => "dir",
            Ok(ft) if ft.is_file() => "file",
            Ok(_) => "other",
            Err(_) => "unknown",
        };
        entries.push(format!("{} ({})", name, file_type));
    }

    entries.sort();
    Ok(entries.join("\n"))
}

pub async fn glob(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: GlobInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let base_path = if let Some(ref p) = input.path {
        ctx.resolve_path(p)?
    } else {
        ctx.working_dir.clone()
    };

    let pattern = base_path.join(&input.pattern);
    let pattern_str = pattern.to_string_lossy();

    // Use bounded heap to keep only the newest MAX_GLOB_RESULTS files
    let mut heap: BinaryHeap<GlobEntry> = BinaryHeap::new();

    for entry in glob_match(&pattern_str)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid glob pattern: {}", e)))?
    {
        match entry {
            Ok(path) if path.is_file() => {
                let mtime = path
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);

                heap.push(GlobEntry { path, mtime });

                // Keep heap bounded
                if heap.len() > MAX_GLOB_RESULTS {
                    // Remove oldest entry by popping and keeping newest
                    let entries: Vec<_> = heap.drain().collect();
                    for entry in entries.into_iter().take(MAX_GLOB_RESULTS) {
                        heap.push(entry);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => {}
        }
    }

    let results: Vec<String> = heap
        .into_sorted_vec()
        .into_iter()
        .rev() // Newest first
        .filter_map(|entry| {
            entry
                .path
                .strip_prefix(&ctx.working_dir)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
        })
        .collect();

    Ok(results.join("\n"))
}

fn is_binary(path: &std::path::Path) -> bool {
    if let Ok(mut file) = std::fs::File::open(path) {
        use std::io::Read;
        let mut buffer = [0u8; BINARY_CHECK_SIZE];
        if let Ok(n) = file.read(&mut buffer) {
            return buffer[..n].contains(&0);
        }
    }
    false
}

pub async fn grep(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: GrepInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let base_path = if let Some(ref p) = input.path {
        ctx.resolve_path(p)?
    } else {
        ctx.working_dir.clone()
    };

    let regex = Regex::new(&input.pattern)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid regex: {}", e)))?;

    let include_pattern = input.include.clone();
    let working_dir = ctx.working_dir.clone();

    // Use spawn_blocking for file I/O intensive operation
    let results = tokio::task::spawn_blocking(move || {
        let mut results = Vec::new();

        for entry in WalkDir::new(&base_path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();

            // Apply include filter
            if let Some(ref pattern) = include_pattern {
                let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !glob::Pattern::new(pattern)
                    .map(|p| p.matches(file_name))
                    .unwrap_or(false)
                {
                    continue;
                }
            }

            // Skip binary files
            if is_binary(path) {
                continue;
            }

            if let Ok(content) = std::fs::read_to_string(path) {
                for (line_num, line) in content.lines().enumerate() {
                    if regex.is_match(line) {
                        let relative_path = path
                            .strip_prefix(&working_dir)
                            .unwrap_or(path)
                            .to_string_lossy();
                        results.push(format!("{}:{}:{}", relative_path, line_num + 1, line));

                        if results.len() >= MAX_GREP_RESULTS {
                            results
                                .push(format!("... (truncated at {} results)", MAX_GREP_RESULTS));
                            return results;
                        }
                    }
                }
            }
        }

        results
    })
    .await
    .map_err(|e| AgentError::ToolExecutionError(format!("Grep task failed: {}", e)))?;

    if results.is_empty() {
        Ok("No matches found".to_string())
    } else {
        Ok(results.join("\n"))
    }
}

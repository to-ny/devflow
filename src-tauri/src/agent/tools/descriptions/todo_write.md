Create and manage a structured task list for the current coding session.

This helps track progress, organize complex tasks, and show the user what you're working on.

## Parameters

- `todos`: Array of task objects (required), where each task has:
  - `id`: Unique identifier (required)
  - `content`: Task description (required)
  - `status`: "pending", "in_progress", or "completed" (required)
  - `priority`: "high", "medium", or "low" (required)

## When to Use

Use this tool proactively for:
1. **Multi-step tasks** - When a task requires 3+ distinct steps
2. **Complex tasks** - Tasks requiring careful planning or multiple operations
3. **Multiple tasks** - When the user provides a list of things to do
4. **New instructions** - Immediately capture requirements as todos
5. **Starting work** - Mark as in_progress BEFORE beginning
6. **Completing work** - Mark as completed immediately when done

## When NOT to Use

Skip this tool when:
- There is only a single, straightforward task
- The task is trivial (can be done in under 3 steps)
- The task is purely conversational or informational

## Task States

- `pending`: Task not yet started
- `in_progress`: Currently working on (limit to ONE at a time)
- `completed`: Task finished successfully

## Critical Rules

1. **Update in real-time** - Change status as you work
2. **One in_progress** - Only one task should be in_progress at a time
3. **Immediate completion** - Mark tasks complete RIGHT AFTER finishing
4. **Only mark complete when FULLY done** - Never mark complete if:
   - Tests are failing
   - Implementation is partial
   - Errors remain unresolved

## Best Practices

- Create specific, actionable items
- Break complex tasks into smaller steps
- Use clear, descriptive task names
- Remove tasks that are no longer relevant

## Example

```json
{
  "todos": [
    {"id": "1", "content": "Read existing auth implementation", "status": "completed", "priority": "high"},
    {"id": "2", "content": "Add logout endpoint", "status": "in_progress", "priority": "high"},
    {"id": "3", "content": "Update frontend to use new endpoint", "status": "pending", "priority": "medium"},
    {"id": "4", "content": "Add tests for logout flow", "status": "pending", "priority": "medium"}
  ]
}
```

Note: The entire list is replaced on each call.

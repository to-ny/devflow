Create or update the task list for this session.

Use when:
- Planning multi-step work
- Tracking progress on complex tasks
- Breaking down large tasks into steps

Input:
- `todos`: Array of task objects with:
  - `id`: Unique identifier
  - `content`: Task description
  - `status`: "pending", "in_progress", or "completed"
  - `priority`: "high", "medium", or "low"

Best practices:
- Update status as you complete tasks
- Keep only one task "in_progress" at a time
- Mark tasks completed immediately when done

The entire list is replaced on each call.

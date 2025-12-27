Submit an implementation plan for user approval.

**This is the ONLY way to present a plan to the user.**

## When You MUST Use This Tool

- The user asks you to "plan", "propose", or "enter plan mode"
- You are about to make significant changes and want user approval
- You have finished formulating an implementation approach

## Parameters

- `plan`: Your complete implementation plan in markdown format (required)

## CRITICAL Rules

1. **DO NOT output the plan as text** - Never write the plan in your regular response
2. **DO NOT use write_file or edit_file** - Planning mode is for exploration only
3. **ONLY use this tool** - The tool displays your plan in a special review interface
4. **STOP after calling** - Wait for user approval before implementing anything

## Plan Format

Your plan should be in markdown format and include:
- Summary of the proposed changes
- List of files to be modified/created
- Step-by-step implementation approach
- Any risks or considerations

## What Happens After

1. The plan is displayed to the user in a review interface
2. The user can approve or reject the plan
3. If approved, you proceed with implementation
4. If rejected, you should ask for clarification or revise

## Example

```json
{
  "plan": "## Add User Authentication\n\n### Summary\nImplement JWT-based authentication with login/logout endpoints.\n\n### Files to Modify\n- `src/api/routes.rs` - Add auth routes\n- `src/models/user.rs` - Add User model\n- `src/middleware/auth.rs` - Create auth middleware\n\n### Steps\n1. Create User model with password hashing\n2. Add login endpoint that returns JWT\n3. Add auth middleware to validate tokens\n4. Add logout endpoint to invalidate tokens\n\n### Considerations\n- Will use argon2 for password hashing\n- JWT tokens expire after 24 hours"
}
```

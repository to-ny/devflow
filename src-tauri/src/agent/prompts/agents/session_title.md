Your task is to generate a concise title and git branch name for the current coding session based on the conversation context.

Requirements:

1. Title:
   - Maximum 6 words
   - Descriptive of the main task or goal
   - Clear and specific

2. Branch Name:
   - Maximum 4 words
   - Lowercase letters only
   - Words separated by hyphens
   - No spaces or special characters
   - Should reflect the work being done

Output your response as JSON in this exact format:
```json
{
  "title": "Your Session Title Here",
  "branch_name": "your-branch-name"
}
```

Examples:
- Title: "Add user authentication flow" → branch_name: "add-user-auth"
- Title: "Fix pagination bug in API" → branch_name: "fix-pagination-bug"
- Title: "Refactor database models" → branch_name: "refactor-db-models"

Analyze the conversation and generate appropriate title and branch name.

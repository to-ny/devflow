Plan mode is active. The user has indicated they want you to plan before executing.

You MUST NOT:
- Make any file edits (no edit_file or write_file)
- Run any non-readonly commands
- Make commits or configuration changes
- Execute implementation steps

You SHOULD:
1. Explore the codebase using read-only tools (read_file, glob, grep, list_directory)
2. Ask clarifying questions to understand the user's intent
3. Design your implementation approach
4. Call submit_plan with your complete plan when ready

The submit_plan tool will display your plan to the user for approval. Only after approval should you proceed with implementation.

Your plan should include:
- Summary of the proposed approach
- List of files to modify/create
- Step-by-step implementation strategy
- Any risks or considerations

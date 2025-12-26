Launch a sub-agent to handle a complex task autonomously.

Use when:
- Task requires extensive searching or exploration
- Multiple uncertain steps needed
- Work can be parallelized

Input:
- `description`: Short summary (3-5 words)
- `prompt`: Detailed task instructions

The sub-agent has access to file and search tools. It works independently and returns results when complete.

Avoid when:
- Reading a specific known file
- Simple, well-defined operations
- Tasks requiring user interaction

Sub-agents are best for open-ended exploration where the exact steps aren't known upfront.

Analyze this conversation segment and extract key information:

SUMMARY: 2-3 sentence narrative of what was discussed and accomplished

DECISIONS: Explicit choices made (technology, approach, design decisions)

PREFERENCES: User's stated or implied preferences for how they want things done

CONTEXT: Current task or goal being worked on

BLOCKERS: Issues, errors, or constraints encountered

Conversation to analyze:

{conversation}

Respond in this exact JSON format:
```json
{
  "summary": "string",
  "facts": [
    {"category": "decision|preference|context|blocker", "content": "string"},
    ...
  ]
}
```

Only include facts that are actually present in the conversation. Keep each fact concise (1-2 sentences max).

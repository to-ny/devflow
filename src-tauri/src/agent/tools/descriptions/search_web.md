Search the web for information and return relevant results.

## Usage

- Performs a web search using the provided query
- Returns search results with titles, URLs, and snippets
- Use for accessing information beyond the knowledge cutoff

## Parameters

- `query`: Search query string (required)
- `allowed_domains`: Array of domains to include exclusively (optional)
- `blocked_domains`: Array of domains to exclude (optional)

## When to Use

- Finding documentation or tutorials
- Researching solutions to problems
- Getting current information about libraries/frameworks
- Looking up error messages or issues
- Finding best practices and patterns

## When NOT to Use

- Fetching a specific known URL (use web_fetch instead)
- Searching local codebase (use grep/glob instead)

## Best Practices

- Use specific, targeted queries for better results
- Include version numbers when searching for library documentation
- Use allowed_domains to focus on authoritative sources
- Use blocked_domains to exclude low-quality results

## Example

```json
{
  "query": "rust tokio async runtime best practices 2024",
  "allowed_domains": ["docs.rs", "tokio.rs", "rust-lang.org"]
}
```

```json
{
  "query": "react hooks useEffect cleanup",
  "blocked_domains": ["w3schools.com"]
}
```

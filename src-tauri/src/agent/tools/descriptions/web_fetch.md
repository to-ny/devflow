Fetches content from a URL and processes it to extract information.

## Usage

- Takes a URL and a prompt describing what to extract
- Fetches the URL content and converts HTML to markdown
- Processes the content with the prompt to extract relevant information
- Returns the processed response about the content

## Parameters

- `url`: Full URL to fetch (required, must be valid HTTPS URL)
- `prompt`: What information to extract from the page (required)

## Features

- HTTP URLs are automatically upgraded to HTTPS
- Results are cached for 15 minutes for faster repeated access
- Large content may be summarized
- HTML is converted to readable markdown

## When to Use

- Reading documentation from the web
- Checking API references
- Fetching external resources
- Getting information from specific web pages

## When NOT to Use

- Reading local files (use read_file instead)
- General web searches (use search_web instead)
- Accessing internal/private URLs

## Handling Redirects

When a URL redirects to a different host, the tool will inform you and provide the redirect URL. Make a new web_fetch request with that URL to fetch the content.

## Example

```json
{
  "url": "https://docs.example.com/api/v2/endpoints",
  "prompt": "List all available API endpoints with their HTTP methods and descriptions"
}
```

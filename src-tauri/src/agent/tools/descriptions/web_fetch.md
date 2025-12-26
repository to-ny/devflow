Fetch a URL and analyze its content.

Use when:
- Reading documentation from the web
- Checking API references
- Fetching external resources

Input:
- `url`: Full URL to fetch (https://)
- `prompt`: What information to extract from the page

The page content is converted to markdown and processed with the prompt. Results are cached for 15 minutes.

HTTP URLs are upgraded to HTTPS automatically.

Avoid for:
- Local files (use read_file)
- General web searches (use web_search)

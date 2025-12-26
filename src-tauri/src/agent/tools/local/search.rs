use scraper::{Html, Selector};

use super::context::{ExecutionContext, MAX_OUTPUT_SIZE};
use crate::agent::error::AgentError;
use crate::agent::tools::types::WebSearchInput;
use crate::config::ConfigService;

const DEFAULT_MAX_RESULTS: usize = 10;

// Browser-like user agent for search to avoid bot detection
const SEARCH_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

#[derive(Debug)]
struct SearchResult {
    title: String,
    url: String,
    snippet: String,
}

pub async fn search(
    ctx: &ExecutionContext,
    input: serde_json::Value,
) -> Result<String, AgentError> {
    let input: WebSearchInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    // Load config to get max_results setting
    let max_results = ConfigService::load_project_config(&ctx.working_dir)
        .map(|c| c.search.max_results as usize)
        .unwrap_or(DEFAULT_MAX_RESULTS);

    let results = duckduckgo_search(ctx, &input.query).await?;

    let filtered: Vec<_> = results
        .into_iter()
        .filter(|r| filter_domain(&r.url, &input.allowed_domains, &input.blocked_domains))
        .take(max_results)
        .collect();

    if filtered.is_empty() {
        return Ok("No results found.".to_string());
    }

    let output = format_results(&filtered);

    Ok(if output.len() > MAX_OUTPUT_SIZE {
        format!(
            "{}...\n(truncated, {} bytes total)",
            &output[..MAX_OUTPUT_SIZE],
            output.len()
        )
    } else {
        output
    })
}

fn filter_domain(url: &str, allowed: &Option<Vec<String>>, blocked: &Option<Vec<String>>) -> bool {
    let domain = extract_domain(url);

    if let Some(allowed_domains) = allowed {
        if !allowed_domains.is_empty() {
            return allowed_domains.iter().any(|d| domain.contains(d));
        }
    }

    if let Some(blocked_domains) = blocked {
        if blocked_domains.iter().any(|d| domain.contains(d)) {
            return false;
        }
    }

    true
}

fn extract_domain(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

fn format_results(results: &[SearchResult]) -> String {
    results
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "{}. **{}**\n   {}\n   {}\n",
                i + 1,
                r.title,
                r.url,
                r.snippet
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

async fn duckduckgo_search(
    ctx: &ExecutionContext,
    query: &str,
) -> Result<Vec<SearchResult>, AgentError> {
    let encoded_query = urlencoding::encode(query);
    let url = format!("https://html.duckduckgo.com/html/?q={}", encoded_query);

    let response = ctx
        .http_client
        .get(&url)
        .header("User-Agent", SEARCH_USER_AGENT)
        .send()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Search request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(AgentError::ToolExecutionError(format!(
            "Search HTTP error: {}",
            status
        )));
    }

    let html = response
        .text()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Failed to read response: {}", e)))?;

    parse_duckduckgo_results(&html)
}

fn parse_duckduckgo_results(html: &str) -> Result<Vec<SearchResult>, AgentError> {
    let document = Html::parse_document(html);

    let result_selector = Selector::parse(".result")
        .map_err(|_| AgentError::ToolExecutionError("Failed to parse selector".to_string()))?;
    let title_selector = Selector::parse(".result__a")
        .map_err(|_| AgentError::ToolExecutionError("Failed to parse selector".to_string()))?;
    let snippet_selector = Selector::parse(".result__snippet")
        .map_err(|_| AgentError::ToolExecutionError("Failed to parse selector".to_string()))?;
    let url_selector = Selector::parse(".result__url")
        .map_err(|_| AgentError::ToolExecutionError("Failed to parse selector".to_string()))?;

    let mut results = Vec::new();

    for result in document.select(&result_selector) {
        let title = result
            .select(&title_selector)
            .next()
            .map(|e| get_text(&e))
            .unwrap_or_default();

        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|e| get_text(&e))
            .unwrap_or_default();

        // Get URL from href or text
        let url = result
            .select(&title_selector)
            .next()
            .and_then(|e| e.value().attr("href"))
            .map(extract_url_from_ddg_href)
            .or_else(|| {
                result
                    .select(&url_selector)
                    .next()
                    .map(|e| format!("https://{}", get_text(&e).trim()))
            })
            .unwrap_or_default();

        if !title.is_empty() && !url.is_empty() {
            results.push(SearchResult {
                title,
                url,
                snippet,
            });
        }
    }

    Ok(results)
}

fn extract_url_from_ddg_href(href: &str) -> String {
    // DDG wraps URLs: //duckduckgo.com/l/?uddg=ENCODED_URL&...
    if href.contains("uddg=") {
        if let Some(start) = href.find("uddg=") {
            let url_part = &href[start + 5..];
            if let Some(end) = url_part.find('&') {
                return urlencoding::decode(&url_part[..end])
                    .map(|s| s.into_owned())
                    .unwrap_or_default();
            } else {
                return urlencoding::decode(url_part)
                    .map(|s| s.into_owned())
                    .unwrap_or_default();
            }
        }
    }

    // Direct URL
    if href.starts_with("http") {
        return href.to_string();
    }

    String::new()
}

fn get_text(element: &scraper::ElementRef) -> String {
    element
        .text()
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

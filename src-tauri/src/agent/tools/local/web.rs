use scraper::{Html, Selector};

use super::context::{ExecutionContext, MAX_OUTPUT_SIZE};
use crate::agent::error::AgentError;
use crate::agent::tools::types::WebFetchInput;

const USER_AGENT: &str = "Mozilla/5.0 (compatible; DevflowBot/1.0)";

pub async fn fetch(ctx: &ExecutionContext, input: serde_json::Value) -> Result<String, AgentError> {
    let input: WebFetchInput = serde_json::from_value(input)
        .map_err(|e| AgentError::InvalidToolInput(format!("Invalid input: {}", e)))?;

    let response = ctx
        .http_client
        .get(&input.url)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Request failed: {}", e)))?;

    let status = response.status();
    if !status.is_success() {
        return Err(AgentError::ToolExecutionError(format!(
            "HTTP error: {}",
            status
        )));
    }

    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let content = response
        .text()
        .await
        .map_err(|e| AgentError::ToolExecutionError(format!("Failed to read response: {}", e)))?;

    let content = if content_type.contains("text/html") || content.trim_start().starts_with("<!") {
        html_to_markdown(&content)
    } else {
        content
    };

    let content = if content.len() > MAX_OUTPUT_SIZE {
        format!(
            "{}...\n(truncated, {} bytes total)",
            &content[..MAX_OUTPUT_SIZE],
            content.len()
        )
    } else {
        content
    };

    Ok(content)
}

fn html_to_markdown(html: &str) -> String {
    let document = Html::parse_document(html);
    let mut output = String::new();

    // Extract title
    if let Some(title) = extract_text(&document, "title") {
        let title = title.trim();
        if !title.is_empty() {
            output.push_str(&format!("# {}\n\n", title));
        }
    }

    // Find main content area
    let content_selectors = [
        "main",
        "article",
        "[role=main]",
        ".content",
        "#content",
        "body",
    ];

    for selector_str in content_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                output.push_str(&element_to_markdown(&element));
                break;
            }
        }
    }

    clean_markdown(&output)
}

fn element_to_markdown(element: &scraper::ElementRef) -> String {
    let mut output = String::new();

    for child in element.children() {
        if let Some(text) = child.value().as_text() {
            let text = text.trim();
            if !text.is_empty() {
                output.push_str(text);
                output.push(' ');
            }
        } else if let Some(el) = scraper::ElementRef::wrap(child) {
            let tag = el.value().name();
            match tag {
                "h1" => output.push_str(&format!("\n# {}\n\n", get_text(&el))),
                "h2" => output.push_str(&format!("\n## {}\n\n", get_text(&el))),
                "h3" => output.push_str(&format!("\n### {}\n\n", get_text(&el))),
                "h4" => output.push_str(&format!("\n#### {}\n\n", get_text(&el))),
                "h5" | "h6" => output.push_str(&format!("\n##### {}\n\n", get_text(&el))),
                "p" => output.push_str(&format!("\n{}\n\n", get_text(&el))),
                "br" => output.push('\n'),
                "hr" => output.push_str("\n---\n\n"),
                "a" => {
                    let href = el.value().attr("href").unwrap_or("#");
                    let text = get_text(&el);
                    if !text.is_empty() {
                        output.push_str(&format!("[{}]({})", text, href));
                    }
                }
                "code" => {
                    let text = get_text(&el);
                    if !text.is_empty() {
                        output.push_str(&format!("`{}`", text));
                    }
                }
                "pre" => output.push_str(&format!("\n```\n{}\n```\n\n", get_text(&el))),
                "strong" | "b" => {
                    let text = get_text(&el);
                    if !text.is_empty() {
                        output.push_str(&format!("**{}**", text));
                    }
                }
                "em" | "i" => {
                    let text = get_text(&el);
                    if !text.is_empty() {
                        output.push_str(&format!("*{}*", text));
                    }
                }
                "ul" => {
                    output.push('\n');
                    if let Ok(li_selector) = Selector::parse("li") {
                        for li in el.select(&li_selector) {
                            output.push_str(&format!("- {}\n", get_text(&li)));
                        }
                    }
                    output.push('\n');
                }
                "ol" => {
                    output.push('\n');
                    if let Ok(li_selector) = Selector::parse("li") {
                        for (i, li) in el.select(&li_selector).enumerate() {
                            output.push_str(&format!("{}. {}\n", i + 1, get_text(&li)));
                        }
                    }
                    output.push('\n');
                }
                "blockquote" => {
                    let text = get_text(&el);
                    for line in text.lines() {
                        output.push_str(&format!("> {}\n", line));
                    }
                    output.push('\n');
                }
                "script" | "style" | "nav" | "footer" | "header" | "aside" | "noscript" => {
                    // Skip non-content elements
                }
                "div" | "section" | "article" | "main" | "span" | "figure" | "figcaption" => {
                    output.push_str(&element_to_markdown(&el));
                }
                "img" => {
                    let alt = el.value().attr("alt").unwrap_or("");
                    let src = el.value().attr("src").unwrap_or("");
                    if !src.is_empty() {
                        output.push_str(&format!("![{}]({})", alt, src));
                    }
                }
                "table" => {
                    output.push_str(&table_to_markdown(&el));
                }
                _ => {
                    let text = get_text(&el);
                    if !text.is_empty() {
                        output.push_str(&text);
                        output.push(' ');
                    }
                }
            }
        }
    }

    output
}

fn table_to_markdown(table: &scraper::ElementRef) -> String {
    let mut output = String::new();
    let row_selector = Selector::parse("tr").unwrap();
    let header_selector = Selector::parse("th").unwrap();
    let cell_selector = Selector::parse("td").unwrap();

    let mut is_first_row = true;
    for row in table.select(&row_selector) {
        let cells: Vec<String> = row
            .select(&header_selector)
            .chain(row.select(&cell_selector))
            .map(|cell| get_text(&cell).replace('|', "\\|"))
            .collect();

        if !cells.is_empty() {
            output.push_str(&format!("| {} |\n", cells.join(" | ")));
            if is_first_row {
                output.push_str(&format!("|{}|\n", " --- |".repeat(cells.len())));
                is_first_row = false;
            }
        }
    }
    output.push('\n');
    output
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

fn extract_text(document: &Html, selector_str: &str) -> Option<String> {
    let selector = Selector::parse(selector_str).ok()?;
    document.select(&selector).next().map(|e| get_text(&e))
}

fn clean_markdown(text: &str) -> String {
    let mut result = String::new();
    let mut prev_empty = false;

    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !prev_empty {
                result.push('\n');
                prev_empty = true;
            }
        } else {
            result.push_str(trimmed);
            result.push('\n');
            prev_empty = false;
        }
    }

    result.trim().to_string()
}

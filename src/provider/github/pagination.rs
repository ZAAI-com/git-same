//! GitHub API pagination handling.
//!
//! GitHub uses Link headers for pagination. This module handles
//! parsing those headers and fetching all pages.

use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde::de::DeserializeOwned;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::errors::ProviderError;

/// Maximum pages to fetch (100 items/page = 10,000 items max).
const MAX_PAGES: usize = 100;

/// Maximum retry attempts for transient failures. Uses exponential backoff.
const MAX_RETRIES: u32 = 3;

/// Initial backoff in ms. Doubles each retry: 1s -> 2s -> 4s.
const INITIAL_BACKOFF_MS: u64 = 1000;

/// Parses the GitHub Link header to find the next page URL.
///
/// GitHub Link headers look like:
/// `<https://api.github.com/user/repos?page=2>; rel="next", <https://api.github.com/user/repos?page=5>; rel="last"`
pub fn parse_link_header(link: &str) -> Option<String> {
    for part in link.split(',') {
        let segments: Vec<&str> = part.split(';').collect();
        if segments.len() >= 2 {
            let rel = segments[1].trim();
            if rel == "rel=\"next\"" {
                let url = segments[0].trim();
                // Remove < and > from URL
                if url.starts_with('<') && url.ends_with('>') {
                    return Some(url[1..url.len() - 1].to_string());
                }
            }
        }
    }
    None
}

/// Format a Unix timestamp as a human-readable reset time string.
fn format_reset_time(reset_timestamp: &str) -> String {
    if let Ok(secs) = reset_timestamp.parse::<i64>() {
        if let Some(dt) = chrono::DateTime::from_timestamp(secs, 0) {
            let wait = dt.signed_duration_since(chrono::Utc::now());
            let mins = wait.num_minutes();
            let secs_rem = wait.num_seconds() % 60;
            return if mins > 0 {
                format!(
                    "{} (resets in {}m {}s)",
                    dt.format("%H:%M:%S UTC"),
                    mins,
                    secs_rem
                )
            } else if secs_rem > 0 {
                format!("{} (resets in {}s)", dt.format("%H:%M:%S UTC"), secs_rem)
            } else {
                format!("{} (resets now)", dt.format("%H:%M:%S UTC"))
            };
        }
    }
    reset_timestamp.to_string()
}

/// Calculate wait time until rate limit reset.
fn calculate_wait_time(reset_timestamp: &str) -> Option<Duration> {
    if let Ok(reset_secs) = reset_timestamp.parse::<u64>() {
        let now = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();

        if reset_secs > now {
            return Some(Duration::from_secs(reset_secs - now));
        }
    }
    None
}

/// Fetches all pages from a GitHub API endpoint using Link header pagination.
///
/// # Arguments
/// * `client` - The HTTP client to use
/// * `token` - The authentication token
/// * `initial_url` - The URL to start fetching from
///
/// This function implements exponential backoff for rate limit errors and transient failures.
pub async fn fetch_all_pages<T: DeserializeOwned>(
    client: &Client,
    token: &str,
    initial_url: &str,
) -> Result<Vec<T>, ProviderError> {
    let mut results = Vec::new();
    let mut url = Some(format!(
        "{}{}per_page=100",
        initial_url,
        if initial_url.contains('?') { "&" } else { "?" }
    ));

    let mut page_count = 0;

    while let Some(current_url) = url {
        let mut retry_count = 0;
        let mut backoff_ms = INITIAL_BACKOFF_MS;

        let (next_url_opt, items) = loop {
            let response = match client
                .get(&current_url)
                .header(AUTHORIZATION, format!("Bearer {}", token))
                .send()
                .await
            {
                Ok(response) => response,
                Err(_) if retry_count < MAX_RETRIES => {
                    retry_count += 1;
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms *= 2;
                    continue;
                }
                Err(e) => return Err(ProviderError::Network(e.to_string())),
            };

            let status = response.status();

            // Check for rate limiting
            if status.as_u16() == 403 {
                if let Some(remaining) = response.headers().get("x-ratelimit-remaining") {
                    if remaining.to_str().unwrap_or("1") == "0" {
                        let reset = response
                            .headers()
                            .get("x-ratelimit-reset")
                            .and_then(|h| h.to_str().ok())
                            .unwrap_or("unknown");

                        // Try to parse reset time and wait
                        if let Some(wait_time) = calculate_wait_time(reset) {
                            if retry_count < MAX_RETRIES {
                                retry_count += 1;
                                // Add a small buffer to the wait time
                                let wait_with_buffer = wait_time + Duration::from_secs(5);
                                tokio::time::sleep(wait_with_buffer).await;
                                continue; // Retry the request
                            }
                        }

                        return Err(ProviderError::RateLimited {
                            reset_time: format_reset_time(reset),
                        });
                    }
                }
            }

            // Retry on 5xx errors with exponential backoff
            if status.is_server_error() && retry_count < MAX_RETRIES {
                retry_count += 1;
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                backoff_ms *= 2; // Exponential backoff: 1s, 2s, 4s
                continue;
            }

            if !status.is_success() {
                let body = response.text().await.unwrap_or_default();
                return Err(ProviderError::from_status(status.as_u16(), body));
            }

            // Get next page URL before consuming response body
            let next_url = response
                .headers()
                .get("Link")
                .and_then(|h| h.to_str().ok())
                .and_then(parse_link_header);

            // Parse response body
            let items: Vec<T> = response
                .json()
                .await
                .map_err(|e| ProviderError::Parse(e.to_string()))?;

            break (next_url, items);
        };

        // Use the next URL from the loop
        url = next_url_opt;

        // Extend results with items from this page
        results.extend(items);

        page_count += 1;
        if page_count >= MAX_PAGES && url.is_some() {
            return Err(ProviderError::Configuration(format!(
                "Pagination truncated after {} pages for '{}'",
                MAX_PAGES, initial_url
            )));
        }
    }

    Ok(results)
}

#[cfg(test)]
#[path = "pagination_tests.rs"]
mod tests;

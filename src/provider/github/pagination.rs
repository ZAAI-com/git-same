//! GitHub API pagination handling.
//!
//! GitHub uses Link headers for pagination. This module handles
//! parsing those headers and fetching all pages.

use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::errors::ProviderError;

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

/// Fetches all pages from a GitHub API endpoint using Link header pagination.
///
/// # Arguments
/// * `client` - The HTTP client to use
/// * `token` - The authentication token
/// * `initial_url` - The URL to start fetching from
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
    const MAX_PAGES: usize = 100; // Safety limit

    while let Some(current_url) = url {
        let response = client
            .get(&current_url)
            .header(AUTHORIZATION, format!("Bearer {}", token))
            .send()
            .await
            .map_err(|e| ProviderError::Network(e.to_string()))?;

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
                    return Err(ProviderError::RateLimited {
                        reset_time: reset.to_string(),
                    });
                }
            }
        }

        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(ProviderError::from_status(status.as_u16(), body));
        }

        // Get next page URL before consuming response body
        url = response
            .headers()
            .get("Link")
            .and_then(|h| h.to_str().ok())
            .and_then(parse_link_header);

        // Parse response body
        let items: Vec<T> = response
            .json()
            .await
            .map_err(|e| ProviderError::Parse(e.to_string()))?;

        results.extend(items);

        page_count += 1;
        if page_count >= MAX_PAGES {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_link_header_with_next() {
        let header = r#"<https://api.github.com/user/repos?page=2>; rel="next", <https://api.github.com/user/repos?page=5>; rel="last""#;
        let next = parse_link_header(header);
        assert_eq!(
            next,
            Some("https://api.github.com/user/repos?page=2".to_string())
        );
    }

    #[test]
    fn test_parse_link_header_without_next() {
        let header = r#"<https://api.github.com/user/repos?page=1>; rel="first", <https://api.github.com/user/repos?page=5>; rel="last""#;
        let next = parse_link_header(header);
        assert_eq!(next, None);
    }

    #[test]
    fn test_parse_link_header_only_last() {
        let header = r#"<https://api.github.com/user/repos?page=1>; rel="prev", <https://api.github.com/user/repos?page=5>; rel="last""#;
        let next = parse_link_header(header);
        assert_eq!(next, None);
    }

    #[test]
    fn test_parse_link_header_empty() {
        let next = parse_link_header("");
        assert_eq!(next, None);
    }

    #[test]
    fn test_parse_link_header_malformed() {
        let header = "malformed header without proper format";
        let next = parse_link_header(header);
        assert_eq!(next, None);
    }

    #[test]
    fn test_parse_link_header_complex() {
        let header = r#"<https://api.github.com/organizations/12345/repos?page=2&per_page=100>; rel="next", <https://api.github.com/organizations/12345/repos?page=10&per_page=100>; rel="last""#;
        let next = parse_link_header(header);
        assert_eq!(
            next,
            Some(
                "https://api.github.com/organizations/12345/repos?page=2&per_page=100".to_string()
            )
        );
    }
}

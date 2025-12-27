#[cfg(feature = "http-poll")]
use super::http_poll_producer::{
  HttpPollProducer, HttpPollProducerConfig, HttpPollResponse, PaginationConfig,
};
#[cfg(feature = "http-poll")]
use async_stream::stream;
#[cfg(feature = "http-poll")]
use async_trait::async_trait;
#[cfg(feature = "http-poll")]
use futures::Stream;
#[cfg(feature = "http-poll")]
use std::pin::Pin;
use streamweave_core::{Producer, ProducerConfig};
use streamweave_error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};

#[cfg(feature = "http-poll")]
use chrono;
#[cfg(feature = "http-poll")]
use std::collections::HashSet;
#[cfg(feature = "http-poll")]
use std::time::{Duration, Instant};
#[cfg(feature = "http-poll")]
use tokio::time::sleep;
#[cfg(feature = "http-poll")]
use tracing::{error, warn};

#[cfg(feature = "http-poll")]
#[allow(clippy::collapsible_if)]
fn create_http_poll_stream(
  client_option: Option<reqwest::Client>,
  http_config: HttpPollProducerConfig,
  component_name: String,
  error_strategy: ErrorStrategy<HttpPollResponse>,
) -> Pin<Box<dyn Stream<Item = HttpPollResponse> + Send>> {
  Box::pin(stream! {
    // Create HTTP client if not already created
    let client = match client_option {
      Some(c) => c,
      None => {
        match reqwest::Client::builder()
          .timeout(http_config.timeout)
          .default_headers(HttpPollResponse::headers_to_reqwest(&http_config.headers))
          .build()
        {
          Ok(c) => c,
          Err(e) => {
            error!(
              component = %component_name,
              error = %e,
              "Failed to create HTTP client, producing empty stream"
            );
            return;
          }
        }
      }
    };

    let mut request_count = 0;
     let mut seen_ids: HashSet<String> = if let Some(ref delta) = http_config.delta_detection {
       delta.seen_ids.clone()
     } else {
       HashSet::new()
     };
      let mut last_request_time: Option<Instant> = None;

    // Poll loop
    loop {
      // Check max requests limit
      if let Some(max) = http_config.max_requests {
        if request_count >= max {
          break;
        }
      }

      // Rate limiting
      if let Some(rps) = http_config.rate_limit {
        if let Some(last_time) = last_request_time {
          let elapsed = last_time.elapsed();
          let min_interval = Duration::from_secs_f64(1.0 / rps);
          if elapsed < min_interval {
            sleep(min_interval - elapsed).await;
          }
        }
      }

      // Make HTTP request
      let request_result = make_request(&client, &http_config, request_count).await;
      last_request_time = Some(Instant::now());
      request_count += 1;

      match request_result {
        Ok((response, _url)) => {
          // Handle pagination
          let mut responses = vec![response];
          if let Some(ref pagination) = http_config.pagination {
            match handle_pagination(&client, &http_config, pagination, request_count).await {
              Ok(mut pages) => {
                responses.append(&mut pages);
              }
              Err(e) => {
                warn!(
                  component = %component_name,
                  error = %e,
                  "Pagination handling failed, emitting initial response only"
                );
              }
            }
          }

          // Process responses
          for mut resp in responses {
            // Delta detection
            if let Some(ref delta_config) = http_config.delta_detection {
              let items = extract_items_from_response(&resp.body, &delta_config.id_field);
              let mut new_items = Vec::new();
              for item in items {
                if let Some(id_value) = item.get(&delta_config.id_field) {
                  let id_str = id_value.to_string();
                  if !seen_ids.contains(&id_str) {
                    seen_ids.insert(id_str.clone());
                    new_items.push(item);
                  }
                }
              }

              // Only emit if there are new items
              if new_items.is_empty() {
                continue;
              }

              // Create response with filtered items
              let mut filtered_body = serde_json::json!({});
              if let Some(obj) = resp.body.as_object_mut() {
                // Try to preserve structure while replacing data array
                for (key, value) in obj.iter() {
                  if key == &delta_config.id_field {
                    continue; // Skip ID field
                  }
                  filtered_body[key] = value.clone();
                }
                filtered_body["items"] = serde_json::Value::Array(
                  new_items.into_iter().map(|item| serde_json::to_value(item).unwrap()).collect()
                );
              }

              let filtered_response = HttpPollResponse::new(
                resp.status,
                resp.headers.clone(),
                filtered_body,
                resp.url.clone(),
              );
              yield filtered_response;
            } else {
              // No delta detection, emit all responses
              yield resp;
            }
          }
        }
        Err(e) => {
          let error = StreamError::new(
            e,
      ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: component_name.clone(),
        component_type: std::any::type_name::<HttpPollProducer>().to_string(),
      },
      ComponentInfo {
        name: component_name.clone(),
        type_name: std::any::type_name::<HttpPollProducer>().to_string(),
      },
          );

          match handle_error_strategy(&error_strategy, &error) {
            ErrorAction::Stop => {
              error!(
                component = %component_name,
                error = %error,
                "Stopping due to HTTP request error"
              );
              break;
            }
            ErrorAction::Skip => {
              warn!(
                component = %component_name,
                error = %error,
                "Skipping due to HTTP request error, continuing to poll"
              );
              sleep(http_config.poll_interval).await;
              continue;
            }
            ErrorAction::Retry => {
              warn!(
                component = %component_name,
                error = %error,
                "Retrying HTTP request after backoff"
              );
              sleep(http_config.retry_backoff).await;
              request_count -= 1; // Don't count failed request
              continue;
            }
          }
        }
      }

      // Wait before next poll
      sleep(http_config.poll_interval).await;
    }
  })
}

#[async_trait]
#[cfg(feature = "http-poll")]
impl Producer for HttpPollProducer {
  type OutputPorts = (crate::http_poll_producer::HttpPollResponse,);

  /// Produces a stream of HTTP responses from polling the configured endpoint.
  ///
  /// # Error Handling
  ///
  /// - Connection errors trigger retries based on the error strategy.
  /// - HTTP errors are handled according to the error strategy.
  /// - Rate limiting is automatically enforced.
  fn produce(&mut self) -> Self::OutputStream {
    let http_config = self.http_config.clone();
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "http_poll_producer".to_string());
    let error_strategy = self.config.error_strategy.clone();

    // Create HTTP client if not already created
    let client_option = self.client.take();

    create_http_poll_stream(client_option, http_config, component_name, error_strategy)
  }

  fn set_config_impl(&mut self, config: ProducerConfig<HttpPollResponse>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &ProducerConfig<HttpPollResponse> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut ProducerConfig<HttpPollResponse> {
    &mut self.config
  }
}

#[cfg(feature = "http-poll")]
async fn make_request(
  client: &reqwest::Client,
  config: &HttpPollProducerConfig,
  request_num: usize,
) -> Result<(HttpPollResponse, String), Box<dyn std::error::Error + Send + Sync>> {
  // Build request URL with pagination if applicable
  let url = build_request_url(config, request_num)?;

  // Build request
  let request = match config.method.as_str() {
    "GET" => client.get(&url),
    "POST" => {
      let mut req = client.post(&url);
      if let Some(ref body) = config.body {
        req = req.body(body.clone());
      }
      req
    }
    "PUT" => {
      let mut req = client.put(&url);
      if let Some(ref body) = config.body {
        req = req.body(body.clone());
      }
      req
    }
    _ => return Err(format!("Unsupported HTTP method: {}", config.method).into()),
  };

  // Execute request with retries
  let mut last_error = None;
  for attempt in 0..=config.retries {
    match request.try_clone() {
      Some(req) => match req.send().await {
        Ok(response) => {
          let status = response.status().as_u16();
          let headers = HttpPollResponse::headers_from_reqwest(response.headers());
          let body = response.json::<serde_json::Value>().await?;

          return Ok((
            HttpPollResponse::new(status, headers, body, url.clone()),
            url,
          ));
        }
        Err(e) => {
          last_error = Some(e);
          if attempt < config.retries {
            sleep(config.retry_backoff * (attempt + 1)).await;
            continue;
          }
        }
      },
      None => {
        // Request cannot be cloned (e.g., has a body), make fresh request
        let req = match config.method.as_str() {
          "GET" => client.get(&url),
          "POST" => {
            let mut r = client.post(&url);
            if let Some(ref body) = config.body {
              r = r.body(body.clone());
            }
            r
          }
          "PUT" => {
            let mut r = client.put(&url);
            if let Some(ref body) = config.body {
              r = r.body(body.clone());
            }
            r
          }
          _ => return Err(format!("Unsupported HTTP method: {}", config.method).into()),
        };

        match req.send().await {
          Ok(response) => {
            let status = response.status().as_u16();
            let headers = HttpPollResponse::headers_from_reqwest(response.headers());
            let body = response.json::<serde_json::Value>().await?;

            return Ok((
              HttpPollResponse::new(status, headers, body, url.clone()),
              url,
            ));
          }
          Err(e) => {
            last_error = Some(e);
            if attempt < config.retries {
              sleep(config.retry_backoff * (attempt + 1)).await;
              continue;
            }
          }
        }
      }
    }
  }

  Err(last_error.unwrap().into())
}

#[cfg(feature = "http-poll")]
fn build_request_url(
  config: &HttpPollProducerConfig,
  request_num: usize,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
  let mut url = config.url.clone();

  if let Some(PaginationConfig::QueryParameter {
    ref page_param,
    limit_param: _,
    step,
    start,
    max_pages: _,
  }) = config.pagination
  {
    let page_value = start + (request_num * step);
    let separator = if url.contains('?') { "&" } else { "?" };
    url.push_str(&format!("{}{}={}", separator, page_param, page_value));
  }

  Ok(url)
}

#[cfg(feature = "http-poll")]
async fn handle_pagination(
  client: &reqwest::Client,
  config: &HttpPollProducerConfig,
  pagination: &PaginationConfig,
  start_request_num: usize,
) -> Result<Vec<HttpPollResponse>, Box<dyn std::error::Error + Send + Sync>> {
  let mut responses = Vec::new();

  match pagination {
    PaginationConfig::QueryParameter {
      page_param: _,
      limit_param: _,
      step: _,
      start: _,
      max_pages,
    } => {
      let pages_to_fetch = max_pages.unwrap_or(usize::MAX);
      for page_num in 1..=pages_to_fetch {
        let url = build_request_url(config, start_request_num + page_num)?;
        let response = client.get(&url).send().await?;
        let status = response.status().as_u16();
        if status >= 400 {
          break; // Stop pagination on error
        }

        let headers = HttpPollResponse::headers_from_reqwest(response.headers());
        let body = response.json::<serde_json::Value>().await?;

        // Check if this page has data
        if !has_data(&body) {
          break;
        }

        // Check for "no more pages" indicator before moving body
        let is_last = is_last_page(&body);
        responses.push(HttpPollResponse::new(status, headers, body, url));

        if is_last {
          break;
        }
      }
    }
    PaginationConfig::LinkHeader => {
      // Start with initial request, then follow Link headers
      let mut next_url = Some(config.url.clone());
      let mut page_count = 0;
      let max_pages = 100; // Safety limit

      while let Some(url) = next_url.take() {
        if page_count >= max_pages {
          break;
        }

        let response = client.get(&url).send().await?;
        let status = response.status().as_u16();
        if status >= 400 {
          break;
        }

        let headers = HttpPollResponse::headers_from_reqwest(response.headers());
        let body = response.json::<serde_json::Value>().await?;

        responses.push(HttpPollResponse::new(
          status,
          headers.clone(),
          body,
          url.clone(),
        ));

        // Extract next URL from Link header
        next_url = extract_next_url_from_link_header(&headers);
        page_count += 1;
      }
    }
    PaginationConfig::JsonField {
      next_field,
      data_field: _,
    } => {
      // Start with initial request, then follow next field
      let mut next_url = Some(config.url.clone());
      let mut page_count = 0;
      let max_pages = 100; // Safety limit

      while let Some(url) = next_url.take() {
        if page_count >= max_pages {
          break;
        }

        let response = client.get(&url).send().await?;
        let status = response.status().as_u16();
        if status >= 400 {
          break;
        }

        let headers = HttpPollResponse::headers_from_reqwest(response.headers());
        let body = response.json::<serde_json::Value>().await?;

        responses.push(HttpPollResponse::new(
          status,
          headers,
          body.clone(),
          url.clone(),
        ));

        // Extract next URL from JSON field
        next_url = body
          .get(next_field)
          .and_then(|v| v.as_str())
          .map(String::from);
        page_count += 1;
      }
    }
  }

  Ok(responses)
}

#[cfg(feature = "http-poll")]
#[allow(clippy::collapsible_if)]
fn extract_items_from_response(
  body: &serde_json::Value,
  id_field: &str,
) -> Vec<serde_json::Map<String, serde_json::Value>> {
  let mut items = Vec::new();

  if let Some(array) = body.as_array() {
    for item in array {
      if let Some(obj) = item.as_object() {
        if obj.contains_key(id_field) {
          items.push(obj.clone());
        }
      }
    }
  } else if let Some(obj) = body.as_object() {
    // Check for common data field names
    for field in &["data", "items", "results", "records"] {
      if let Some(array) = obj.get(*field).and_then(|v| v.as_array()) {
        for item in array {
          if let Some(item_obj) = item.as_object()
            && item_obj.contains_key(id_field)
          {
            items.push(item_obj.clone());
          }
        }
      }
    }
  }

  items
}

#[cfg(feature = "http-poll")]
#[allow(clippy::collapsible_if)]
fn has_data(body: &serde_json::Value) -> bool {
  if let Some(array) = body.as_array() {
    !array.is_empty()
  } else if let Some(obj) = body.as_object() {
    // Check common data fields
    for field in &["data", "items", "results", "records"] {
      if let Some(array) = obj.get(*field).and_then(|v| v.as_array()) {
        if !array.is_empty() {
          return true;
        }
      }
    }
    false
  } else {
    false
  }
}

#[cfg(feature = "http-poll")]
#[allow(clippy::collapsible_if)]
fn is_last_page(body: &serde_json::Value) -> bool {
  // Check for common pagination indicators
  if let Some(obj) = body.as_object() {
    if let Some(has_next) = obj.get("has_next").and_then(|v| v.as_bool()) {
      return !has_next;
    }
    if let Some(next) = obj.get("next") {
      return next.is_null();
    }
    if let Some(page) = obj.get("page").and_then(|v| v.as_u64()) {
      if let Some(total_pages) = obj.get("total_pages").and_then(|v| v.as_u64()) {
        return page >= total_pages;
      }
    }
  }
  false
}

#[cfg(feature = "http-poll")]
fn extract_next_url_from_link_header(
  headers: &std::collections::HashMap<String, String>,
) -> Option<String> {
  headers.get("link").and_then(|link_value| {
    // Parse Link header: <url>; rel="next"
    for link in link_value.split(',') {
      let parts: Vec<&str> = link.split(';').collect();
      if parts.len() >= 2 {
        let url = parts[0].trim().trim_matches('<').trim_matches('>');
        let rel = parts[1].trim();
        if rel.contains("rel=\"next\"") || rel.contains("rel='next'") {
          return Some(url.to_string());
        }
      }
    }
    None
  })
}

#[cfg(feature = "http-poll")]
fn handle_error_strategy<T>(strategy: &ErrorStrategy<T>, error: &StreamError<T>) -> ErrorAction
where
  T: std::fmt::Debug + Clone + Send + Sync,
{
  match strategy {
    ErrorStrategy::Stop => ErrorAction::Stop,
    ErrorStrategy::Skip => ErrorAction::Skip,
    ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
    ErrorStrategy::Custom(handler) => handler(error),
    _ => ErrorAction::Stop,
  }
}

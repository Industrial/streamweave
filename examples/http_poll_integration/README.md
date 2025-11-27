# HTTP Polling Integration Example

This example demonstrates how to use StreamWeave's HTTP polling producer to poll HTTP endpoints with support for pagination, delta detection, and rate limiting. This is useful for integrating with APIs that don't support webhooks.

## Prerequisites

Before running this example, you need:

1. **HTTP polling feature enabled** - Build with `--features http-poll`
2. **Internet connection** - Examples use public APIs (JSONPlaceholder) for demonstration

## Quick Start

```bash
# Run basic polling example
cargo run --example http_poll_integration --features http-poll basic

# Run pagination example
cargo run --example http_poll_integration --features http-poll pagination

# Run delta detection example
cargo run --example http_poll_integration --features http-poll delta

# Run rate-limited polling example
cargo run --example http_poll_integration --features http-poll rate-limit
```

## Running Examples

### 1. Basic HTTP Polling

This example demonstrates simple HTTP polling at a configurable interval:

```bash
cargo run --example http_poll_integration --features http-poll basic
```

**What it demonstrates:**
- Polling an HTTP endpoint at regular intervals
- Handling HTTP responses
- Basic error handling with skip strategy
- Configurable polling interval and max requests

**Configuration:**
- Polls `https://jsonplaceholder.typicode.com/posts/1` every 2 seconds
- Makes 3 requests then stops
- Extracts and displays post titles

### 2. Pagination Handling

This example demonstrates automatic pagination handling:

```bash
cargo run --example http_poll_integration --features http-poll pagination
```

**What it demonstrates:**
- Query parameter-based pagination (`_page` and `_limit`)
- Automatically fetching multiple pages
- Handling paginated JSON responses
- Configuring page size and max pages

**Configuration:**
- Uses JSONPlaceholder API with `_page` and `_limit` parameters
- Fetches 3 pages automatically
- Extracts post titles from each page

**Pagination Strategies Supported:**
- **Query Parameters**: `?page=1&limit=10` (demonstrated here)
- **Link Headers**: `Link: <next-url>; rel="next"`
- **JSON Fields**: `{ "next": "...", "data": [...] }`

### 3. Delta Detection

This example demonstrates tracking seen items and emitting only new ones:

```bash
cargo run --example http_poll_integration --features http-poll delta
```

**What it demonstrates:**
- Tracking items by unique ID field
- Emitting only new items on subsequent polls
- Maintaining state across polling cycles
- Reducing duplicate processing

**Configuration:**
- Tracks posts by `id` field
- First poll emits all items
- Subsequent polls emit only new items (in this case, none since API returns same data)

**Use Cases:**
- Polling APIs that return all items each time
- Reducing downstream processing load
- Implementing change detection

### 4. Rate-Limited Polling

This example demonstrates rate limiting to respect API limits:

```bash
cargo run --example http_poll_integration --features http-poll rate-limit
```

**What it demonstrates:**
- Configuring requests per second limit
- Automatic throttling to respect rate limits
- Measuring actual request timing
- Combining with polling intervals

**Configuration:**
- Polls every 100ms (attempted)
- Rate limited to 2 requests per second (actual)
- Makes 5 requests total
- Should take ~2.5 seconds total

**Use Cases:**
- Respecting API rate limits
- Avoiding overwhelming external services
- Implementing backpressure

## Example Output

### Basic Polling
```
🚀 StreamWeave HTTP Polling Integration Example
==============================================

Running: Basic HTTP Polling
---------------------------
📡 Setting up basic HTTP polling...
🔄 Starting polling (3 requests, 2 second intervals)...
✅ Polling completed!
📊 Results (3 items):
  1. Post: sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  2. Post: sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  3. Post: sunt aut facere repellat provident occaecati excepturi optio reprehenderit
```

### Pagination
```
🚀 StreamWeave HTTP Polling Integration Example
==============================================

Running: Pagination Handling
-----------------------------
📡 Setting up pagination example...
🔄 Starting paginated polling (3 pages)...
✅ Pagination completed!
📊 Results (3 pages):
  Page 1: Page with 10 posts: sunt aut facere..., qui est esse..., ea molestias...
  Page 2: Page with 10 posts: eum et est..., nesciunt quas odio..., dolorem eum...
  Page 3: Page with 10 posts: optio molestias..., et ea vero..., in quibusdam...
```

### Delta Detection
```
🚀 StreamWeave HTTP Polling Integration Example
==============================================

Running: Delta Detection
------------------------
📡 Setting up delta detection example...
🔄 Starting delta detection polling (2 requests)...
   First request will emit all items, second will emit only new ones
✅ Delta detection completed!
📊 New items detected (10 items):
  1. [ID: 1] sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  2. [ID: 2] qui est esse
  ...
```

### Rate-Limited Polling
```
🚀 StreamWeave HTTP Polling Integration Example
==============================================

Running: Rate Limited Polling
------------------------------
📡 Setting up rate-limited polling...
🔄 Starting rate-limited polling (5 requests, max 2 req/sec)...
✅ Rate-limited polling completed!
⏱️  Total time: 2.51s (should be ~2.5s for 5 requests at 2 req/sec)
📊 Results (5 items):
  1. [200] sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  2. [200] sunt aut facere repellat provident occaecati excepturi optio reprehenderit
  ...
```

## Configuration Options

### HttpPollProducerConfig

The HTTP polling producer supports extensive configuration:

```rust
HttpPollProducerConfig::default()
    .with_url("https://api.example.com/data")
    .with_poll_interval(Duration::from_secs(60))
    .with_max_requests(Some(100))  // None for infinite
    .with_rate_limit(10.0)        // 10 requests per second
    .with_timeout(Duration::from_secs(30))
    .with_retries(3)
    .with_retry_backoff(Duration::from_secs(1))
    .with_pagination(pagination_config)
    .with_delta_detection(delta_config)
```

### PaginationConfig

**Query Parameter Pagination:**
```rust
PaginationConfig::QueryParameter {
    page_param: "page".to_string(),
    limit_param: Some("limit".to_string()),
    step: 1,
    start: 1,
    max_pages: Some(10),
}
```

**Link Header Pagination:**
```rust
PaginationConfig::LinkHeader
```

**JSON Field Pagination:**
```rust
PaginationConfig::JsonField {
    next_field: "next".to_string(),
    data_field: "data".to_string(),
}
```

### DeltaDetectionConfig

```rust
DeltaDetectionConfig::new("id")  // Track by "id" field

// Or with existing seen IDs
DeltaDetectionConfig::with_seen_ids("id", seen_ids_set)
```

## Error Handling

The HTTP polling producer supports different error strategies:

- **Stop**: Stop polling on first error
- **Skip**: Skip failed requests and continue
- **Retry**: Retry failed requests with backoff

```rust
.with_error_strategy(ErrorStrategy::Skip)
```

## Use Cases

1. **API Integration**: Poll REST APIs that don't support webhooks
2. **Change Detection**: Monitor APIs for new or updated items
3. **Data Synchronization**: Keep local data in sync with remote APIs
4. **Event Collection**: Collect events from APIs that only support polling
5. **Rate-Limited APIs**: Respect API rate limits automatically

## Best Practices

1. **Set Appropriate Intervals**: Don't poll too frequently; respect API guidelines
2. **Use Rate Limiting**: Always configure rate limits to avoid overwhelming APIs
3. **Handle Errors Gracefully**: Use Skip or Retry strategies based on your needs
4. **Use Delta Detection**: For APIs that return all items, use delta detection to avoid reprocessing
5. **Set Timeouts**: Always configure timeouts to avoid hanging requests
6. **Monitor Polling**: Track request counts and timing to optimize configuration

## Troubleshooting

### Connection Errors

If you see connection errors:
- Check internet connectivity
- Verify the API endpoint is accessible
- Check firewall/proxy settings
- Increase timeout if API is slow

### Rate Limiting Issues

If requests are being throttled:
- Increase `poll_interval` to reduce request frequency
- Decrease `rate_limit` to respect API limits
- Check API documentation for rate limit guidelines

### Pagination Not Working

If pagination isn't working:
- Verify the API supports the pagination method you're using
- Check response format matches your configuration
- Enable logging to see actual requests/responses

## Next Steps

- Explore other StreamWeave examples
- Integrate HTTP polling with transformers and consumers
- Combine with other producers for complex data pipelines
- See the main StreamWeave documentation for advanced features


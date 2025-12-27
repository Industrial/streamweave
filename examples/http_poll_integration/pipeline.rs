#[cfg(feature = "http-poll")]
use serde::{Deserialize, Serialize};
#[cfg(feature = "http-poll")]
use std::time::Duration;
#[cfg(feature = "http-poll")]
use streamweave::{
  consumers::vec::vec_consumer::VecConsumer,
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::http_poll::http_poll_producer::{
    DeltaDetectionConfig, HttpPollProducer, HttpPollProducerConfig, HttpPollResponse,
    PaginationConfig,
  },
  transformers::flatten::flatten_transformer::FlattenTransformer,
  transformers::map::map_transformer::MapTransformer,
};

/// Example: Basic HTTP polling
///
/// This demonstrates:
/// - Polling an HTTP endpoint at a configurable interval
/// - Handling HTTP responses
/// - Basic error handling
#[cfg(feature = "http-poll")]
pub async fn basic_polling() -> Result<(), Box<dyn std::error::Error>> {
  println!("📡 Setting up basic HTTP polling...");

  // Configure HTTP polling producer
  // Using JSONPlaceholder API as a public test API
  let http_config = HttpPollProducerConfig::default()
    .with_url("https://jsonplaceholder.typicode.com/posts/1")
    .with_poll_interval(Duration::from_secs(2))
    .with_max_requests(3) // Poll 3 times then stop
    .with_timeout(Duration::from_secs(10));

  let producer = HttpPollProducer::new(http_config)
    .with_name("http-poll-basic".to_string())
    .with_error_strategy(ErrorStrategy::Skip); // Skip on errors

  // Transform responses to extract useful data
  let transformer = MapTransformer::new(|response: HttpPollResponse| -> Result<String, String> {
    // Extract title from the response body if it's a post
    if let Some(title) = response.body.get("title").and_then(|v| v.as_str()) {
      Ok(format!("Post: {}", title))
    } else {
      Ok(format!("Response: {}", response.body))
    }
  });

  // Collect results
  let consumer = VecConsumer::new();

  // Build and run pipeline
  println!("🔄 Starting polling (3 requests, 2 second intervals)...");
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();

  println!("\n✅ Polling completed!");
  println!("📊 Results ({} items):", results.len());
  for (i, result) in results.iter().enumerate() {
    match result {
      Ok(value) => println!("  {}. {}", i + 1, value),
      Err(e) => println!("  {}. Error: {}", i + 1, e),
    }
  }

  Ok(())
}

/// Example: Pagination handling
///
/// This demonstrates:
/// - Query parameter-based pagination
/// - Fetching multiple pages automatically
/// - Handling paginated responses
#[cfg(feature = "http-poll")]
pub async fn pagination_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📡 Setting up pagination example...");

  // Configure pagination using query parameters
  // JSONPlaceholder supports pagination via _page and _limit
  let pagination = PaginationConfig::QueryParameter {
    page_param: "_page".to_string(),
    limit_param: Some("_limit".to_string()),
    step: 1,
    start: 1,
    max_pages: Some(3), // Fetch 3 pages
  };

  let http_config = HttpPollProducerConfig::default()
    .with_url("https://jsonplaceholder.typicode.com/posts")
    .with_poll_interval(Duration::from_secs(1))
    .with_max_requests(1) // Only one initial request, pagination handles the rest
    .with_pagination(pagination)
    .with_timeout(Duration::from_secs(10));

  let producer = HttpPollProducer::new(http_config)
    .with_name("http-poll-pagination".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transform to extract post titles from paginated responses
  let transformer = MapTransformer::new(
    |response: HttpPollResponse| -> Result<Vec<String>, String> {
      // Extract array of posts from response
      if let Some(posts) = response.body.as_array() {
        let titles: Vec<String> = posts
          .iter()
          .filter_map(|post| post.get("title").and_then(|t| t.as_str()))
          .map(|s| s.to_string())
          .collect();
        Ok(titles)
      } else {
        Err("Expected array in response".to_string())
      }
    },
  );

  // Unwrap Result<Vec<String>, String> to Vec<String> for FlattenTransformer
  let unwrap_transformer =
    MapTransformer::new(|result: Result<Vec<String>, String>| -> Vec<String> {
      result.unwrap_or_default()
    });

  // Flatten Vec<String> to individual String items
  let flatten_transformer = FlattenTransformer::<String>::new();

  // Format each title for display
  let format_transformer = MapTransformer::new(|title: String| -> Result<String, String> {
    Ok(format!("Post: {}", title))
  });

  let consumer = VecConsumer::new();

  println!("🔄 Starting paginated polling (3 pages)...");
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .transformer(unwrap_transformer)
    .transformer(flatten_transformer)
    .transformer(format_transformer)
    .consumer(consumer);

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();

  println!("\n✅ Pagination completed!");
  println!("📊 Results ({} posts across all pages):", results.len());
  for (i, result) in results.iter().enumerate() {
    match result {
      Ok(value) => println!("  {}. {}", i + 1, value),
      Err(e) => println!("  {}. Error: {}", i + 1, e),
    }
  }

  Ok(())
}

/// Example: Delta detection (only emit new items)
///
/// This demonstrates:
/// - Tracking seen items by ID
/// - Emitting only new items
/// - Maintaining state across polls
#[cfg(feature = "http-poll")]
pub async fn delta_detection_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📡 Setting up delta detection example...");

  // Configure delta detection to track items by "id" field
  let delta_detection = DeltaDetectionConfig::new("id");

  let http_config = HttpPollProducerConfig::default()
    .with_url("https://jsonplaceholder.typicode.com/posts")
    .with_poll_interval(Duration::from_secs(2))
    .with_max_requests(2) // Poll twice to demonstrate delta detection
    .with_delta_detection(delta_detection)
    .with_timeout(Duration::from_secs(10));

  let producer = HttpPollProducer::new(http_config)
    .with_name("http-poll-delta".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transform to extract individual posts
  let transformer =
    MapTransformer::new(|response: HttpPollResponse| -> Result<Vec<Post>, String> {
      if let Some(posts) = response.body.as_array() {
        let posts: Result<Vec<Post>, _> = posts
          .iter()
          .map(|p| serde_json::from_value(p.clone()))
          .collect();
        posts.map_err(|e| format!("Failed to parse post: {}", e))
      } else {
        Err("Expected array in response".to_string())
      }
    });

  // Unwrap Result<Vec<Post>, String> to Vec<Post> for FlattenTransformer
  let unwrap_transformer = MapTransformer::new(|result: Result<Vec<Post>, String>| -> Vec<Post> {
    result.unwrap_or_default()
  });

  // Flatten Vec<Post> to individual Post items
  let flatten_transformer = FlattenTransformer::<Post>::new();

  let consumer = VecConsumer::new();

  println!("🔄 Starting delta detection polling (2 requests)...");
  println!("   First request will emit all items, second will emit only new ones");
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .transformer(unwrap_transformer)
    .transformer(flatten_transformer)
    .consumer(consumer);

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();

  println!("\n✅ Delta detection completed!");
  println!("📊 New items detected ({} items):", results.len());
  for (i, post) in results.iter().enumerate() {
    println!("  {}. [ID: {}] {}", i + 1, post.id, post.title);
  }

  Ok(())
}

/// Example: Rate-limited polling
///
/// This demonstrates:
/// - Configuring rate limits (requests per second)
/// - Respecting API rate limits
/// - Throttling requests automatically
#[cfg(feature = "http-poll")]
pub async fn rate_limited_polling() -> Result<(), Box<dyn std::error::Error>> {
  println!("📡 Setting up rate-limited polling...");

  // Configure rate limiting: 2 requests per second
  let http_config = HttpPollProducerConfig::default()
    .with_url("https://jsonplaceholder.typicode.com/posts/1")
    .with_poll_interval(Duration::from_millis(100)) // Try to poll every 100ms
    .with_rate_limit(2.0) // But limit to 2 requests per second
    .with_max_requests(5) // Make 5 requests
    .with_timeout(Duration::from_secs(10));

  let producer = HttpPollProducer::new(http_config)
    .with_name("http-poll-rate-limited".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let transformer = MapTransformer::new(|response: HttpPollResponse| -> Result<String, String> {
    if let Some(title) = response.body.get("title").and_then(|v| v.as_str()) {
      Ok(format!("[{}] {}", response.status, title))
    } else {
      Ok(format!("[{}] Response received", response.status))
    }
  });

  let consumer = VecConsumer::new();

  println!("🔄 Starting rate-limited polling (5 requests, max 2 req/sec)...");
  let start = std::time::Instant::now();
  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(consumer);

  let (_, result_consumer) = pipeline.run().await?;
  let results = result_consumer.into_vec();
  let elapsed = start.elapsed();

  println!("\n✅ Rate-limited polling completed!");
  println!(
    "⏱️  Total time: {:.2}s (should be ~2.5s for 5 requests at 2 req/sec)",
    elapsed.as_secs_f64()
  );
  println!("📊 Results ({} items):", results.len());
  for (i, result) in results.iter().enumerate() {
    match result {
      Ok(value) => println!("  {}. {}", i + 1, value),
      Err(e) => println!("  {}. Error: {}", i + 1, e),
    }
  }

  Ok(())
}

/// Simple post structure for delta detection example
#[cfg(feature = "http-poll")]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Post {
  id: u32,
  title: String,
  body: String,
  #[serde(rename = "userId")]
  user_id: u32,
}

#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use sqlx::SqlitePool;
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
use streamweave::{
  consumers::console::console_consumer::ConsoleConsumer,
  error::ErrorStrategy,
  pipeline::PipelineBuilder,
  producers::database::database_producer::{
    DatabaseProducer, DatabaseProducerConfig, DatabaseRow, DatabaseType,
  },
  transformers::map::map_transformer::MapTransformer,
};

/// Example: Basic database query with in-memory SQLite
///
/// This demonstrates:
/// - Creating an in-memory SQLite database
/// - Executing a simple SELECT query
/// - Streaming query results through a pipeline
/// - Displaying results to the console
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
pub async fn basic_query_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up basic database query example...");

  // Create an in-memory SQLite database and populate it with sample data
  let pool = setup_test_database().await?;

  // Configure the database producer
  let db_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared".to_string())
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email, age FROM users ORDER BY id")
    .with_max_connections(5)
    .with_fetch_size(100); // Fetch 100 rows at a time for cursor-based streaming

  let producer = DatabaseProducer::new(db_config)
    .with_name("database_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip); // Skip rows that fail to process

  // Transform database rows to formatted strings for display
  let transformer = MapTransformer::new(|row: DatabaseRow| -> String {
    let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("N/A");
    let email = row.get("email").and_then(|v| v.as_str()).unwrap_or("N/A");
    let age = row.get("age").and_then(|v| v.as_i64()).unwrap_or(0);
    format!("User #{}: {} ({}) - Age: {}", id, name, email, age)
  })
  .with_error_strategy(ErrorStrategy::Skip);

  println!("🚀 Starting pipeline to query database...");
  println!("   Database: SQLite (in-memory)");
  println!("   Query: SELECT id, name, email, age FROM users");
  println!();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(ConsoleConsumer::new());

  // Run the pipeline
  pipeline.run().await?;

  // Keep the pool alive until the end
  drop(pool);

  println!("\n✅ Basic query example completed!");
  Ok(())
}

/// Example: Parameterized queries
///
/// This demonstrates:
/// - Using parameterized queries to prevent SQL injection
/// - Binding parameters to queries
/// - Filtering results based on parameters
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
pub async fn parameterized_query_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up parameterized query example...");

  // Create an in-memory SQLite database and populate it with sample data
  let pool = setup_test_database().await?;

  // Configure the database producer with a parameterized query
  // SQLite uses ? for parameter placeholders
  let db_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared".to_string())
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email, age FROM users WHERE age > ? ORDER BY age DESC")
    .with_parameter(serde_json::Value::Number(25.into())) // Only users older than 25
    .with_max_connections(5)
    .with_fetch_size(50);

  let producer = DatabaseProducer::new(db_config)
    .with_name("parameterized_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Transform and format results
  let transformer = MapTransformer::new(|row: DatabaseRow| -> String {
    let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
    let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("N/A");
    let age = row.get("age").and_then(|v| v.as_i64()).unwrap_or(0);
    format!("User #{}: {} (Age: {})", id, name, age)
  })
  .with_error_strategy(ErrorStrategy::Skip);

  println!("🚀 Starting pipeline with parameterized query...");
  println!("   Query: SELECT * FROM users WHERE age > ?");
  println!("   Parameter: 25");
  println!();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(transformer)
    .consumer(ConsoleConsumer::new());

  pipeline.run().await?;

  drop(pool);

  println!("\n✅ Parameterized query example completed!");
  Ok(())
}

/// Example: Streaming large result sets
///
/// This demonstrates:
/// - Cursor-based streaming for large datasets
/// - Memory-efficient processing of large result sets
/// - Connection pooling configuration
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
pub async fn large_result_set_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up large result set streaming example...");

  // Create an in-memory SQLite database and populate it with a large dataset
  let pool = setup_large_test_database().await?;

  // Configure the database producer with optimized settings for large results
  let db_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared".to_string())
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email, age FROM large_users ORDER BY id")
    .with_max_connections(10) // More connections for better throughput
    .with_fetch_size(500); // Larger fetch size for better performance

  let producer = DatabaseProducer::new(db_config)
    .with_name("large_result_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  // Count rows as they stream through (demonstrates memory efficiency)
  use streamweave::consumers::vec::vec_consumer::VecConsumer;
  let consumer = VecConsumer::new();

  println!("🚀 Starting pipeline to stream large result set...");
  println!("   Database: SQLite (in-memory)");
  println!("   Query: SELECT * FROM large_users");
  println!("   Fetch size: 500 rows per batch");
  println!("   Max connections: 10");
  println!("   Processing...");

  // Add identity transformer to pass through database rows unchanged
  use streamweave::producers::database::database_producer::DatabaseRow;
  let identity_transformer = MapTransformer::new(|row: DatabaseRow| -> Result<DatabaseRow, String> {
    Ok(row)
  });

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(identity_transformer)
    .consumer(consumer);

  let ((), result) = pipeline.run().await?;

  println!(
    "\n✅ Processed {} rows from large result set!",
    result.vec.len()
  );
  println!("   Memory usage remains bounded due to cursor-based streaming");

  drop(pool);

  println!("\n✅ Large result set example completed!");
  Ok(())
}

/// Example: Connection pooling configuration
///
/// This demonstrates:
/// - Configuring connection pool settings
/// - Understanding pool behavior
/// - Optimizing for different workloads
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
pub async fn connection_pooling_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📊 Setting up connection pooling example...");

  // Create an in-memory SQLite database
  let pool = setup_test_database().await?;

  // Configure with custom connection pool settings
  let db_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared".to_string())
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email FROM users")
    .with_max_connections(20) // Maximum connections in the pool
    .with_fetch_size(1000); // Larger fetch size for batch processing

  let producer = DatabaseProducer::new(db_config)
    .with_name("pooled_producer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  println!("🚀 Starting pipeline with custom connection pool...");
  println!("   Max connections: 20");
  println!("   Fetch size: 1000 rows per batch");
  println!("   Connection pool manages connections efficiently");
  println!();

  // Add identity transformer to pass through database rows unchanged
  let identity_transformer = MapTransformer::new(|row: DatabaseRow| -> Result<DatabaseRow, String> {
    Ok(row)
  });

  // Use VecConsumer instead of ConsoleConsumer since we have Result types
  use streamweave::consumers::vec::vec_consumer::VecConsumer;
  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .producer(producer)
    .transformer(identity_transformer)
    .consumer(consumer);

  pipeline.run().await?;

  drop(pool);

  println!("\n✅ Connection pooling example completed!");
  Ok(())
}

/// Helper function to set up a test database with sample data
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
async fn setup_test_database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
  // Create an in-memory SQLite database
  // Using "?cache=shared" allows multiple connections to the same in-memory database
  let pool = SqlitePool::connect("sqlite::memory:?cache=shared").await?;

  // Create the users table
  sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS users (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL,
      email TEXT NOT NULL,
      age INTEGER NOT NULL
    )
    "#,
  )
  .execute(&pool)
  .await?;

  // Insert sample data
  sqlx::query(
    r#"
    INSERT INTO users (name, email, age) VALUES
      ('Alice', 'alice@example.com', 30),
      ('Bob', 'bob@example.com', 25),
      ('Charlie', 'charlie@example.com', 35),
      ('Diana', 'diana@example.com', 28),
      ('Eve', 'eve@example.com', 32)
    "#,
  )
  .execute(&pool)
  .await?;

  Ok(pool)
}

/// Helper function to set up a test database with a large dataset
#[cfg(all(not(target_arch = "wasm32"), feature = "database"))]
async fn setup_large_test_database() -> Result<SqlitePool, Box<dyn std::error::Error>> {
  // Create an in-memory SQLite database
  let pool = SqlitePool::connect("sqlite::memory:?cache=shared").await?;

  // Create the large_users table
  sqlx::query(
    r#"
    CREATE TABLE IF NOT EXISTS large_users (
      id INTEGER PRIMARY KEY AUTOINCREMENT,
      name TEXT NOT NULL,
      email TEXT NOT NULL,
      age INTEGER NOT NULL
    )
    "#,
  )
  .execute(&pool)
  .await?;

  // Insert a large number of rows (1000 rows)
  // This demonstrates cursor-based streaming for large result sets
  for i in 1..=1000 {
    sqlx::query(
      r#"
      INSERT INTO large_users (name, email, age) VALUES (?, ?, ?)
      "#,
    )
    .bind(format!("User{}", i))
    .bind(format!("user{}@example.com", i))
    .bind(20 + (i % 50)) // Age between 20 and 69
    .execute(&pool)
    .await?;
  }

  println!("   Created 1000 rows in large_users table");

  Ok(pool)
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "database")))]
#[allow(dead_code)]
pub async fn basic_query_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Database feature is not enabled. Build with --features database".into())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "database")))]
#[allow(dead_code)]
pub async fn parameterized_query_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Database feature is not enabled. Build with --features database".into())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "database")))]
#[allow(dead_code)]
pub async fn large_result_set_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Database feature is not enabled. Build with --features database".into())
}

#[cfg(not(all(not(target_arch = "wasm32"), feature = "database")))]
#[allow(dead_code)]
pub async fn connection_pooling_example() -> Result<(), Box<dyn std::error::Error>> {
  Err("Database feature is not enabled. Build with --features database".into())
}

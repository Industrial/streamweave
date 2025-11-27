//! # SQL Dialect Example
//!
//! This example demonstrates the Stream SQL dialect design for StreamWeave.
//! It shows the supported SQL constructs and their semantics for stream processing.
//!
//! Note: This example demonstrates the dialect design. Actual parsing and
//! execution will be implemented in subsequent tasks.

#[cfg(feature = "sql")]
use streamweave::sql::dialect::StreamSqlDialect;

#[cfg(feature = "sql")]
fn main() {
  println!("StreamWeave SQL Dialect Examples");
  println!("=================================\n");

  // Example 1: Simple SELECT with WHERE
  println!("Example 1: Simple Filtering");
  println!("SQL: SELECT user_id, event_type, value FROM events WHERE value > 100");
  println!("Semantics: Filters stream elements as they arrive\n");

  // Example 2: Aggregation with GROUP BY and WINDOW
  println!("Example 2: Time-Based Aggregation");
  println!("SQL: SELECT user_id, COUNT(*) as purchase_count, SUM(amount) as total_spent");
  println!("      FROM purchases");
  println!("      WHERE timestamp > NOW() - INTERVAL '1 day'");
  println!("      GROUP BY user_id");
  println!("      WINDOW TUMBLING (SIZE 1 HOUR)");
  println!("Semantics: Produces results when windows close\n");

  // Example 3: Stream Join
  println!("Example 3: Stream Join");
  println!("SQL: SELECT u.user_id, u.name, p.product_name, p.amount");
  println!("      FROM users u");
  println!("      JOIN purchases p ON u.user_id = p.user_id");
  println!("      WINDOW TUMBLING (SIZE 5 MINUTES)");
  println!("Semantics: Joins elements within the same window\n");

  // Demonstrate dialect configuration
  println!("Dialect Configuration:");
  let dialect = StreamSqlDialect::default();
  println!(
    "  Allow unbounded ORDER BY: {}",
    dialect.allow_unbounded_order_by
  );
  println!(
    "  Max window size (seconds): {:?}",
    dialect.max_window_size_seconds
  );
  println!(
    "  Max window size (rows): {:?}",
    dialect.max_window_size_rows
  );

  // Demonstrate window size validation
  println!("\nWindow Size Validation:");
  match dialect.validate_window_size(Some(3600), None) {
    Ok(_) => println!("  ✓ 1 hour window: Valid"),
    Err(e) => println!("  ✗ 1 hour window: {}", e),
  }

  match dialect.validate_window_size(Some(100000), None) {
    Ok(_) => println!("  ✓ 100000 second window: Valid"),
    Err(e) => println!("  ✗ 100000 second window: {}", e),
  }

  println!("\n✅ SQL Dialect design demonstrated!");
  println!("\nNote: Actual SQL parsing and execution will be implemented in Tasks 12.2-12.4");
}

#[cfg(not(feature = "sql"))]
fn main() {
  eprintln!("❌ Error: SQL feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'sql' feature to be enabled.");
  eprintln!("Build with: cargo run --example sql_dialect --features sql");
  std::process::exit(1);
}

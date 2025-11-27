//! # SQL Parser Example
//!
//! This example demonstrates the SQL parser functionality for StreamWeave.
//! It shows how SQL queries are parsed into ASTs that can be translated
//! into StreamWeave pipelines.

#[cfg(feature = "sql")]
use streamweave::sql::SqlParser;

#[cfg(feature = "sql")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("StreamWeave SQL Parser Examples");
  println!("=================================\n");

  let parser = SqlParser::new();

  // Example 1: Simple SELECT
  println!("Example 1: Simple SELECT");
  println!("SQL: SELECT id, name FROM users");
  match parser.parse("SELECT id, name FROM users") {
    Ok(query) => {
      println!("✓ Parsed successfully!");
      println!("  Stream: {}", query.from.stream);
      println!("  Selected items: {}", query.select.items.len());
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 2: SELECT with WHERE
  println!("Example 2: SELECT with WHERE");
  println!("SQL: SELECT * FROM events WHERE value > 100");
  match parser.parse("SELECT * FROM events WHERE value > 100") {
    Ok(query) => {
      println!("✓ Parsed successfully!");
      println!("  Has WHERE clause: {}", query.where_clause.is_some());
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 3: SELECT with GROUP BY
  println!("Example 3: SELECT with GROUP BY");
  println!("SQL: SELECT user_id, COUNT(*) FROM events GROUP BY user_id");
  match parser.parse("SELECT user_id, COUNT(*) FROM events GROUP BY user_id") {
    Ok(query) => {
      println!("✓ Parsed successfully!");
      println!("  Has GROUP BY: {}", query.group_by.is_some());
      if let Some(group_by) = &query.group_by {
        println!("  Group by keys: {}", group_by.keys.len());
      }
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 4: SELECT with ORDER BY and LIMIT
  println!("Example 4: SELECT with ORDER BY and LIMIT");
  println!("SQL: SELECT * FROM events ORDER BY timestamp DESC LIMIT 100");
  match parser.parse("SELECT * FROM events ORDER BY timestamp DESC LIMIT 100") {
    Ok(query) => {
      println!("✓ Parsed successfully!");
      println!("  Has ORDER BY: {}", query.order_by.is_some());
      println!("  Has LIMIT: {}", query.limit.is_some());
      if let Some(limit) = &query.limit {
        println!("  Limit count: {}", limit.count);
      }
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 5: Error case - invalid syntax
  println!("Example 5: Error Handling");
  println!("SQL: SELECT FROM (invalid syntax)");
  match parser.parse("SELECT FROM") {
    Ok(_) => println!("✗ Unexpectedly parsed invalid query"),
    Err(e) => {
      println!("✓ Correctly rejected invalid query");
      println!("  Error: {}", e);
    }
  }
  println!();

  println!("✅ SQL Parser examples completed!");
  println!("\nNote: Query translation to pipelines will be implemented in Task 12.3");

  Ok(())
}

#[cfg(not(feature = "sql"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  eprintln!("❌ Error: SQL feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'sql' feature to be enabled.");
  eprintln!("Build with: cargo run --example sql_parser --features sql");
  std::process::exit(1);
}

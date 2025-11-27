//! # SQL Query Example
//!
//! This example demonstrates the complete SQL query functionality:
//! parsing, translation, and optimization.

#[cfg(feature = "sql")]
use streamweave::sql::{QueryOptimizer, QueryTranslator, parse_query};

#[cfg(feature = "sql")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("StreamWeave SQL Query Example");
  println!("==============================\n");

  // Example 1: Parse a simple query
  println!("Example 1: Parse SQL Query");
  println!("SQL: SELECT user_id, COUNT(*) FROM events GROUP BY user_id");
  match parse_query("SELECT user_id, COUNT(*) FROM events GROUP BY user_id") {
    Ok(query) => {
      println!("✓ Query parsed successfully!");
      println!("  Stream: {}", query.from.stream);
      println!("  Selected items: {}", query.select.items.len());
      if let Some(group_by) = &query.group_by {
        println!("  Group by keys: {}", group_by.keys.len());
      }
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 2: Translate query to plan
  println!("Example 2: Translate Query to Plan");
  match parse_query("SELECT * FROM events WHERE value > 100 ORDER BY timestamp DESC LIMIT 10") {
    Ok(query) => {
      let translator = QueryTranslator::new();
      match translator.translate(&query) {
        Ok(plan) => {
          println!("✓ Query plan created!");
          println!("  Source stream: {}", plan.source_stream);
          println!("  Operations: {}", plan.operations.len());
          for (i, op) in plan.operations.iter().enumerate() {
            println!("    {}. {:?}", i + 1, op);
          }
        }
        Err(e) => println!("✗ Translation error: {}", e),
      }
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  // Example 3: Optimize query
  println!("Example 3: Optimize Query");
  match parse_query("SELECT id FROM users WHERE 5 + 3 > 7") {
    Ok(query) => {
      let optimizer = QueryOptimizer::new();
      match optimizer.optimize(query) {
        Ok(optimized) => {
          println!("✓ Query optimized!");
          if let Some(where_clause) = &optimized.where_clause {
            println!("  Optimized WHERE: {}", where_clause.condition);
            // Constant folding should have evaluated 5 + 3 to 8
          }
        }
        Err(e) => println!("✗ Optimization error: {}", e),
      }
    }
    Err(e) => println!("✗ Parse error: {}", e),
  }
  println!();

  println!("✅ SQL Query examples completed!");
  println!("\nNote: Full pipeline execution requires a producer to be provided");
  println!("for the stream specified in the FROM clause.");

  Ok(())
}

#[cfg(not(feature = "sql"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
  eprintln!("❌ Error: SQL feature is not enabled");
  eprintln!();
  eprintln!("This example requires the 'sql' feature to be enabled.");
  eprintln!("Build with: cargo run --example sql_query --features sql");
  std::process::exit(1);
}

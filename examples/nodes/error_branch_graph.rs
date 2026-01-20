//! # Error Branch Graph Example
//!
//! This example demonstrates how to use GraphBuilder to construct a graph and ErrorBranchNode
//! for routing successful calculations vs errors.
//! **No custom nodes required** - this example uses only existing nodes from the standard library.
//!
//! The example:
//! 1. Creates a graph with DivideNode (which can produce division-by-zero errors) and ErrorBranchNode
//! 2. Uses GraphBuilder to connect nodes in the graph structure
//! 3. Demonstrates ErrorBranchNode by executing it directly with test data
//! 4. Shows how successful results vs errors are routed to different output ports
//! 5. Displays meaningful results showing error routing behavior
//!
//! ## Running the Example
//!
//! ```bash
//! cargo run --example error_branch_graph
//! ```

use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph_builder::GraphBuilder;
use streamweave::node::Node;
use streamweave::nodes::arithmetic::DivideNode;
use streamweave::nodes::error_branch_node::ErrorBranchNode;
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  println!("Error Branch Graph Example");
  println!("==========================");
  println!("This example demonstrates ErrorBranchNode routing calculations with errors.");
  println!();

  // Demonstrate GraphBuilder by building a graph structure
  println!("Building graph structure with GraphBuilder...");

  let _graph = GraphBuilder::new("calculator_with_error_handling")
    // Add nodes
    .add_node("divide", Box::new(DivideNode::new("divide".to_string())))
    .add_node(
      "error_branch",
      Box::new(ErrorBranchNode::new("error_branch".to_string())),
    )
    // Connect nodes: divide results go to error_branch
    .connect("divide", "out", "error_branch", "in")
    .connect("divide", "error", "error_branch", "in")
    .build()
    .map_err(|e| format!("Failed to build graph: {}", e))?;

  println!("Graph built successfully!");
  println!("Nodes: divide, error_branch");
  println!("Connections: divide (out, error) → error_branch (in)");
  println!();

  // Now demonstrate ErrorBranchNode directly
  println!("Demonstrating ErrorBranchNode functionality...");
  let error_branch_node = ErrorBranchNode::new("result_router".to_string());

  // Create input streams for the ErrorBranchNode
  let (_config_tx, config_rx) = mpsc::channel(10);
  let (data_tx, data_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx)) as streamweave::node::InputStream,
  );
  inputs.insert(
    "in".to_string(),
    Box::pin(ReceiverStream::new(data_rx)) as streamweave::node::InputStream,
  );

  // Execute the ErrorBranchNode
  println!("Executing ErrorBranchNode...");
  let outputs_future = error_branch_node.execute(inputs);
  let mut outputs = outputs_future
    .await
    .map_err(|e| format!("Node execution failed: {:?}", e))?;

  // Send test data to ErrorBranchNode
  println!("Sending test data to ErrorBranchNode...");

  // Create test data: some successful calculations, some errors
  let test_data = vec![
    // Successful results (wrapped as Ok)
    Ok::<Arc<dyn Any + Send + Sync>, String>(Arc::new(42i32)),
    Ok::<Arc<dyn Any + Send + Sync>, String>(Arc::new("Success!".to_string())),
    // Errors (wrapped as Err)
    Err::<Arc<dyn Any + Send + Sync>, String>("Division by zero".to_string()),
    Err::<Arc<dyn Any + Send + Sync>, String>("Invalid input type".to_string()),
    // Another success
    Ok::<Arc<dyn Any + Send + Sync>, String>(Arc::new(100i32)),
  ];

  println!(
    "Sending {} test items: {} successes, {} errors",
    test_data.len(),
    3,
    2
  );

  for (i, result) in test_data.into_iter().enumerate() {
    let description = match &result {
      Ok(_) => format!("Success #{}", i + 1),
      Err(_) => format!("Error #{}", i + 1),
    };
    println!("  {}", description);
    data_tx
      .send(Arc::new(result) as Arc<dyn Any + Send + Sync>)
      .await?;
  }

  // Small delay to let processing complete
  tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

  // Collect results from success port
  println!();
  println!("Results from 'success' port:");
  let success_stream = outputs.remove("success").unwrap();
  let mut success_count = 0;
  let mut success_stream = success_stream;

  for _ in 0..3 {
    // Expect 3 successes
    tokio::select! {
        result = success_stream.next() => {
            if let Some(item) = result {
                success_count += 1;
                // Try to downcast to different types to show what we received
                if let Ok(value) = item.clone().downcast::<i32>() {
                    println!("  Success: integer {}", *value);
                } else if let Ok(value) = item.clone().downcast::<String>() {
                    println!("  Success: string '{}'", *value);
                } else {
                    println!("  Success: {} (unknown type)", std::any::type_name_of_val(&*item));
                }
            } else {
                break;
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            break;
        }
    }
  }

  // Collect results from error port
  println!("Results from 'error' port:");
  let error_stream = outputs.remove("error").unwrap();
  let mut error_count = 0;
  let mut error_stream = error_stream;

  for _ in 0..2 {
    // Expect 2 errors
    tokio::select! {
        result = error_stream.next() => {
            if let Some(item) = result {
                error_count += 1;
                if let Ok(error_msg) = item.clone().downcast::<String>() {
                    println!("  Error: {}", error_msg);
                } else {
                    println!("  Error: {} (unknown type)", std::any::type_name_of_val(&*item));
                }
            } else {
                break;
            }
        }
        _ = tokio::time::sleep(tokio::time::Duration::from_millis(100)) => {
            break;
        }
    }
  }

  // Summary
  println!();
  println!("Summary:");
  println!("- Items routed to 'success' port: {}", success_count);
  println!("- Items routed to 'error' port: {}", error_count);

  // Verify expected results
  let expected_successes = 3;
  let expected_errors = 2;

  if success_count == expected_successes && error_count == expected_errors {
    println!("✅ ErrorBranchNode correctly routed all results!");
    println!("   - Successes: {} ✅", success_count);
    println!("   - Errors: {} ✅", error_count);
  } else {
    println!("❌ Some routing may have failed:");
    println!(
      "   - Expected {} successes, got {}",
      expected_successes, success_count
    );
    println!(
      "   - Expected {} errors, got {}",
      expected_errors, error_count
    );
  }

  println!();
  println!("GraphBuilder and ErrorBranchNode example completed successfully!");

  Ok(())
}

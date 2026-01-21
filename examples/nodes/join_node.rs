use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use streamweave::node::Node;
use streamweave::nodes::join_node::{join_config, JoinConfig, JoinNode, JoinStrategy};
use tokio::sync::mpsc;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

// Simple user data structure
#[derive(Clone, Debug)]
struct User {
  id: i32,
  name: String,
}

// Simple order data structure
#[derive(Clone, Debug)]
struct Order {
  user_id: i32,
  amount: f64,
}

// Combined user-order data structure
#[derive(Clone, Debug)]
struct UserOrder {
  user_name: String,
  order_amount: f64,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {

  // Create join configuration for inner join
  let join_config: Arc<JoinConfig> = join_config(
    JoinStrategy::Inner,
    // Left key extractor (User.id)
    |value| {
      let fut = async move {
        if let Ok(user_arc) = value.downcast::<User>() {
          Ok(user_arc.id.to_string())
        } else {
          Err("Expected User".to_string())
        }
      };
      Box::pin(fut) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
    },
    // Right key extractor (Order.user_id)
    |value| {
      let fut = async move {
        if let Ok(order_arc) = value.downcast::<Order>() {
          Ok(order_arc.user_id.to_string())
        } else {
          Err("Expected Order".to_string())
        }
      };
      Box::pin(fut) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
    },
    // Combine function
    |left, right| {
      let fut = async move {
        let user_arc = left.downcast::<User>()
          .map_err(|_| "Expected User in left".to_string())?;

        if let Some(order_arc) = right {
          let order = order_arc.downcast::<Order>()
            .map_err(|_| "Expected Order in right".to_string())?;

          let user_order = UserOrder {
            user_name: user_arc.name.clone(),
            order_amount: order.amount,
          };
          Ok(Arc::new(user_order) as Arc<dyn Any + Send + Sync>)
        } else {
          // For inner join, we shouldn't get None, but handle it gracefully
          Err("Inner join requires matching items".to_string())
        }
      };
      Box::pin(fut) as std::pin::Pin<Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>>
    },
  );

  // Create the JoinNode directly
  let join_node = JoinNode::new("user_order_join".to_string());

  // Create input streams for the JoinNode
  let (config_tx, config_rx) = mpsc::channel(10);
  let (left_tx, left_rx) = mpsc::channel(10);
  let (right_tx, right_rx) = mpsc::channel(10);

  let mut inputs = HashMap::new();
  inputs.insert(
    "configuration".to_string(),
    Box::pin(ReceiverStream::new(config_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "left".to_string(),
    Box::pin(ReceiverStream::new(left_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );
  inputs.insert(
    "right".to_string(),
    Box::pin(ReceiverStream::new(right_rx))
      as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
  );

  println!("✓ JoinNode created with input streams");

  // Send configuration
  let _ = config_tx
    .send(Arc::new(join_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send user data (left stream)
  let users = vec![
    User { id: 1, name: "Alice".to_string() },
    User { id: 2, name: "Bob".to_string() },
    User { id: 3, name: "Charlie".to_string() },
  ];

  for user in users {
    let _ = left_tx
      .send(Arc::new(user) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  // Send order data (right stream)
  let orders = vec![
    Order { user_id: 1, amount: 100.0 }, // Alice's order
    Order { user_id: 2, amount: 250.0 }, // Bob's order
    Order { user_id: 1, amount: 75.0 },  // Alice's second order
    Order { user_id: 4, amount: 50.0 },  // No matching user (will be ignored in inner join)
  ];

  for order in orders {
    let _ = right_tx
      .send(Arc::new(order) as Arc<dyn Any + Send + Sync>)
      .await;
  }

  println!("✓ Data sent to streams");

  // Drop the transmitters to close the input channels (signals EOF to streams)
  // This must happen BEFORE calling execute, so the streams know when input ends
  drop(config_tx);
  drop(left_tx);
  drop(right_tx);

  // Execute the JoinNode
  println!("Executing JoinNode...");
  let start = std::time::Instant::now();
  let outputs_future = join_node.execute(inputs);
  let mut outputs = outputs_future
    .await
    .map_err(|e| format!("JoinNode execution failed: {:?}", e))?;
  println!("✓ JoinNode execution completed in {:?}", start.elapsed());

  // Read joined results from the output stream
  println!("Reading joined results from output stream...");
  let mut result_count = 0;
  if let Some(mut output_stream) = outputs.remove("out") {
    while let Some(item) = output_stream.next().await {
      if let Ok(user_order_arc) = item.downcast::<UserOrder>() {
        let user_order = &*user_order_arc;
        println!(
          "  Joined: {} ordered ${:.2}",
          user_order.user_name, user_order.order_amount
        );
        result_count += 1;
      }
    }
  }

  // Read errors from the error stream (should be empty for this example)
  println!("Reading errors from error stream...");
  let mut error_count = 0;
  if let Some(mut error_stream) = outputs.remove("error") {
    while let Some(item) = error_stream.next().await {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = &**error_msg;
        println!("  Error: {}", error);
        error_count += 1;
      }
    }
  }

  println!(
    "✓ Received {} joined results via output channel",
    result_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  Ok(())
}

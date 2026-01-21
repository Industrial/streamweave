use std::any::Any;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::join_node::{JoinConfig, JoinNode, JoinStrategy, join_config};
use tokio::sync::mpsc;

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
  // Create channels for external I/O
  let (config_tx, config_rx) = mpsc::channel(1);
  let (left_tx, left_rx) = mpsc::channel(10);
  let (right_tx, right_rx) = mpsc::channel(10);
  let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
  let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

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
      Box::pin(fut)
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
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
      Box::pin(fut)
        as std::pin::Pin<Box<dyn std::future::Future<Output = Result<String, String>> + Send>>
    },
    // Combine function
    |left, right| {
      let fut = async move {
        let user_arc = left
          .downcast::<User>()
          .map_err(|_| "Expected User in left".to_string())?;

        if let Some(order_arc) = right {
          let order = order_arc
            .downcast::<Order>()
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
      Box::pin(fut)
        as std::pin::Pin<
          Box<dyn std::future::Future<Output = Result<Arc<dyn Any + Send + Sync>, String>> + Send>,
        >
    },
  );

  // Build the graph using the Graph API
  let mut graph = Graph::new("user_order_join_example".to_string());
  graph.add_node(
    "join".to_string(),
    Box::new(JoinNode::new("join".to_string())),
  )?;
  graph.expose_input_port("join", "configuration", "configuration")?;
  graph.expose_input_port("join", "left", "left")?;
  graph.expose_input_port("join", "right", "right")?;
  graph.expose_output_port("join", "out", "output")?;
  graph.expose_output_port("join", "error", "error")?;
  graph.connect_input_channel("configuration", config_rx)?;
  graph.connect_input_channel("left", left_rx)?;
  graph.connect_input_channel("right", right_rx)?;
  graph.connect_output_channel("output", out_tx)?;
  graph.connect_output_channel("error", error_tx)?;

  println!("✓ Graph built with JoinNode using Graph API");

  // Send configuration
  let _ = config_tx
    .send(Arc::new(join_config) as Arc<dyn Any + Send + Sync>)
    .await;

  // Send user data (left stream)
  let users = vec![
    User {
      id: 1,
      name: "Alice".to_string(),
    },
    User {
      id: 2,
      name: "Bob".to_string(),
    },
    User {
      id: 3,
      name: "Charlie".to_string(),
    },
  ];

  for user in users {
    let _ = left_tx
      .send(Arc::new(user) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  // Send order data (right stream)
  let orders = vec![
    Order {
      user_id: 1,
      amount: 100.0,
    }, // Alice's order
    Order {
      user_id: 2,
      amount: 250.0,
    }, // Bob's order
    Order {
      user_id: 1,
      amount: 75.0,
    }, // Alice's second order
    Order {
      user_id: 4,
      amount: 50.0,
    }, // No matching user (will be ignored in inner join)
  ];

  for order in orders {
    let _ = right_tx
      .send(Arc::new(order) as Arc<dyn Any + Send + Sync>)
      .await;
    tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
  }

  println!("✓ Configuration sent and test data sent to input channels");

  // Execute the graph
  println!("Executing graph with JoinNode...");
  let start = std::time::Instant::now();
  graph
    .execute()
    .await
    .map_err(|e| format!("Graph execution failed: {:?}", e))?;
  println!("✓ Graph execution completed in {:?}", start.elapsed());

  // Drop the transmitters to close the input channels (signals EOF to streams)
  drop(config_tx);
  drop(left_tx);
  drop(right_tx);

  // Read joined results from the output channels
  println!("Reading joined results from output channels...");
  let mut result_count = 0;
  let mut error_count = 0;

  loop {
    let out_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
    let error_result =
      tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

    let mut has_data = false;

    if let Ok(Some(item)) = out_result {
      if let Ok(user_order_arc) = item.downcast::<UserOrder>() {
        let user_order = &*user_order_arc;
        println!(
          "  Joined: {} ordered ${:.2}",
          user_order.user_name, user_order.order_amount
        );
        result_count += 1;
        has_data = true;
      }
    }

    if let Ok(Some(item)) = error_result {
      if let Ok(error_msg) = item.downcast::<String>() {
        let error = (**error_msg).to_string();
        println!("  Error: {}", error);
        error_count += 1;
        has_data = true;
      }
    }

    if !has_data {
      break;
    }
  }

  println!(
    "✓ Received {} joined results via output channel",
    result_count
  );
  println!("✓ Received {} errors via error channel", error_count);
  println!("✓ Total completed in {:?}", start.elapsed());

  // Verify behavior: should receive 3 joined results (Alice: 2 orders, Bob: 1 order)
  if result_count == 3 && error_count == 0 {
    println!("✓ JoinNode correctly performed inner join on user_id");
  } else {
    println!(
      "⚠ JoinNode behavior may be unexpected (results: {}, errors: {}, expected results: 3, errors: 0)",
      result_count, error_count
    );
  }

  Ok(())
}

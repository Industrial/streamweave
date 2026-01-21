use std::any::Any;
use std::collections::HashMap;
use std::sync::Arc;
use streamweave::graph::Graph;
use streamweave::nodes::write_variable_node::{WriteVariableConfig, WriteVariableNode};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels for external I/O
    let (config_tx, config_rx) = mpsc::channel(1);
    let (name_tx, name_rx) = mpsc::channel(5);
    let (value_tx, value_rx) = mpsc::channel(5);
    let (out_tx, mut out_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
    let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

    // Create a variable store and share it with the node
    let variable_store: WriteVariableConfig = Arc::new(tokio::sync::Mutex::new(HashMap::new()));

    // Build the graph directly with connected channels
    let mut graph = Graph::new("variable_writer".to_string());
    graph.add_node("write_var".to_string(), Box::new(WriteVariableNode::new("write_var".to_string())))?;
    graph.expose_input_port("write_var", "configuration", "configuration")?;
    graph.expose_input_port("write_var", "name", "name")?;
    graph.expose_input_port("write_var", "value", "value")?;
    graph.expose_output_port("write_var", "out", "out")?;
    graph.expose_output_port("write_var", "error", "error")?;
    graph.connect_input_channel("configuration", config_rx)?;
    graph.connect_input_channel("name", name_rx)?;
    graph.connect_input_channel("value", value_rx)?;
    graph.connect_output_channel("out", out_tx)?;
    graph.connect_output_channel("error", error_tx)?;

    println!("✓ Graph built with connected channels");

    // Send configuration and input data to the channels AFTER building
    let _ = config_tx
        .send(Arc::new(variable_store.clone()) as Arc<dyn Any + Send + Sync>)
        .await;

    let variables_to_write = vec![
        ("user_count", Arc::new(42i32) as Arc<dyn Any + Send + Sync>),
        ("server_name", Arc::new("production-server".to_string()) as Arc<dyn Any + Send + Sync>),
        ("pi_value", Arc::new(3.14159f64) as Arc<dyn Any + Send + Sync>),
        ("is_active", Arc::new(true) as Arc<dyn Any + Send + Sync>),
    ];

    for (var_name, var_value) in variables_to_write {
        // Send the variable name first
        let _ = name_tx
            .send(Arc::new(var_name.to_string()) as Arc<dyn Any + Send + Sync>)
            .await;
        // Then send the corresponding value
        let _ = value_tx.send(var_value).await;
    }

    // Send one more name but no corresponding value (should error)
    let _ = name_tx
        .send(Arc::new("orphaned_name".to_string()) as Arc<dyn Any + Send + Sync>)
        .await;

    println!("✓ Data sent to channels");

    // Execute the graph (channels have data now)
    println!("Executing graph...");
    let start = std::time::Instant::now();
    graph
        .execute()
        .await
        .map_err(|e| format!("Graph execution failed: {:?}", e))?;
    println!("✓ Graph execution completed in {:?}", start.elapsed());

    // Drop the transmitters to close the input channels (signals EOF to streams)
    drop(config_tx);
    drop(name_tx);
    drop(value_tx);

    // Read results from the output channels
    println!("Reading results from output channels...");
    let mut write_count = 0;
    let mut error_count = 0;

    loop {
        let out_result = tokio::time::timeout(tokio::time::Duration::from_millis(100), out_rx.recv()).await;
        let error_result = tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

        let mut has_data = false;

        if let Ok(Some(item)) = out_result {
            // Try to downcast to different types
            if let Ok(int_arc) = item.clone().downcast::<i32>() {
                let value = *int_arc;
                println!("  Write confirmation (i32): {}", value);
                write_count += 1;
                has_data = true;
            } else if let Ok(string_arc) = item.clone().downcast::<String>() {
                let value = &**string_arc;
                println!("  Write confirmation (String): {}", value);
                write_count += 1;
                has_data = true;
            } else if let Ok(float_arc) = item.clone().downcast::<f64>() {
                let value = *float_arc;
                println!("  Write confirmation (f64): {}", value);
                write_count += 1;
                has_data = true;
            } else if let Ok(bool_arc) = item.downcast::<bool>() {
                let value = *bool_arc;
                println!("  Write confirmation (bool): {}", value);
                write_count += 1;
                has_data = true;
            }
        }

        if let Ok(Some(item)) = error_result {
            if let Ok(error_msg) = item.downcast::<String>() {
                let error = &**error_msg;
                println!("  Error: {}", error);
                error_count += 1;
                has_data = true;
            }
        }

        if !has_data {
            break;
        }
    }

    println!("✓ Received {} write confirmations via output channel", write_count);
    println!("✓ Received {} errors via error channel", error_count);

    // Verify the variables were actually written by reading from the store
    println!("Verifying variables in the store...");
    let store = variable_store.lock().await;
    for (key, value) in store.iter() {
        if let Ok(int_arc) = value.clone().downcast::<i32>() {
            let val = *int_arc;
            println!("  {} = {} (i32)", key, val);
        } else if let Ok(string_arc) = value.clone().downcast::<String>() {
            let val = &**string_arc;
            println!("  {} = {} (String)", key, val);
        } else if let Ok(float_arc) = value.clone().downcast::<f64>() {
            let val = *float_arc;
            println!("  {} = {} (f64)", key, val);
        } else if let Ok(bool_arc) = value.clone().downcast::<bool>() {
            let val = *bool_arc;
            println!("  {} = {} (bool)", key, val);
        }
    }
    println!("✓ Store contains {} variables", store.len());

    println!("✓ Total completed in {:?}", start.elapsed());

    Ok(())
}

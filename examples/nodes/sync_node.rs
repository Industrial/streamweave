use std::any::Any;
use std::sync::Arc;
use std::time::Duration;
use streamweave::graph::Graph;
use streamweave::nodes::sync_node::{SyncConfig, SyncNode};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create channels for external I/O
    let (config_tx, config_rx) = mpsc::channel(1);
    let (in0_tx, in0_rx) = mpsc::channel(5);
    let (in1_tx, in1_rx) = mpsc::channel(5);
    let (in2_tx, in2_rx) = mpsc::channel(5);
    let (output_tx, mut output_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);
    let (error_tx, mut error_rx) = mpsc::channel::<Arc<dyn Any + Send + Sync>>(10);

    // Create configuration with timeout
    let sync_config = SyncConfig {
        num_inputs: 3,
        timeout: Some(Duration::from_millis(500)), // 500ms timeout
    };

    // Build the graph directly with connected channels
    let mut graph = Graph::new("data_sync".to_string());
    graph.add_node("sync".to_string(), Box::new(SyncNode::new("sync".to_string(), 3)))?;
    graph.expose_input_port("sync", "configuration", "configuration")?;
    graph.expose_input_port("sync", "in_0", "in_0")?;
    graph.expose_input_port("sync", "in_1", "in_1")?;
    graph.expose_input_port("sync", "in_2", "in_2")?;
    graph.expose_output_port("sync", "out", "output")?;
    graph.expose_output_port("sync", "error", "error")?;
    graph.connect_input_channel("configuration", config_rx)?;
    graph.connect_input_channel("in_0", in0_rx)?;
    graph.connect_input_channel("in_1", in1_rx)?;
    graph.connect_input_channel("in_2", in2_rx)?;
    graph.connect_output_channel("output", output_tx)?;
    graph.connect_output_channel("error", error_tx)?;

    println!("✓ Graph built with connected channels");

    // Send configuration and input data to the channels AFTER building
    let _ = config_tx
        .send(Arc::new(sync_config) as Arc<dyn Any + Send + Sync>)
        .await;

    let _ = in0_tx
        .send(Arc::new("data_0".to_string()) as Arc<dyn Any + Send + Sync>)
        .await;
    let _ = in1_tx
        .send(Arc::new(42i32) as Arc<dyn Any + Send + Sync>)
        .await;
    let _ = in2_tx
        .send(Arc::new(3.14f64) as Arc<dyn Any + Send + Sync>)
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
    drop(in0_tx);
    drop(in1_tx);
    drop(in2_tx);

    // Read results from the output channels
    println!("Reading results from output channels...");
    let mut output_count = 0;
    let mut error_count = 0;

    let mut output_rx = output_rx;
    let mut error_rx = error_rx;

    loop {
        let output_result = tokio::time::timeout(tokio::time::Duration::from_millis(100), output_rx.recv()).await;
        let error_result = tokio::time::timeout(tokio::time::Duration::from_millis(100), error_rx.recv()).await;

        let mut has_data = false;

        if let Ok(Some(item)) = output_result {
            if let Ok(vec_arc) = item.downcast::<Vec<Arc<dyn Any + Send + Sync>>>() {
                let combined = vec_arc;
                println!("  Synchronized data ({} items):", combined.len());
                for (i, value) in combined.iter().enumerate() {
                    if let Ok(string_arc) = value.clone().downcast::<String>() {
                        let s = &**string_arc;
                        println!("    [{}] {}", i, s);
                    } else if let Ok(int_arc) = value.clone().downcast::<i32>() {
                        let n = *int_arc;
                        println!("    [{}] {}", i, n);
                    } else if let Ok(float_arc) = value.clone().downcast::<f64>() {
                        let f = *float_arc;
                        println!("    [{}] {}", i, f);
                    }
                }
                output_count += 1;
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

    println!("✓ Received {} synchronized outputs via output channel", output_count);
    println!("✓ Received {} errors via error channel", error_count);
    println!("✓ Total completed in {:?}", start.elapsed());

    Ok(())
}

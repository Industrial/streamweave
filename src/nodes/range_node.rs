//! # Range Node
//!
//! A source node that generates number sequences (ranges) for iteration.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives configuration updates (optional, for future extensibility)
//! - **Input**: `"start"` - Receives the starting value of the range
//! - **Input**: `"end"` - Receives the ending value of the range (exclusive)
//! - **Input**: `"step"` - Receives the step size for the range
//! - **Output**: `"out"` - Sends generated numbers from the range
//! - **Output**: `"error"` - Sends errors that occur during range generation
//!
//! ## Behavior
//!
//! The node waits for `start`, `end`, and `step` values to be received. Once all three are available,
//! it generates a sequence of numbers from `start` to `end` (exclusive) with the specified `step` size.
//! Each number in the sequence is emitted as a separate output on the `out` port.
//!
//! The node supports both integer and floating-point ranges. The type is inferred from the first
//! value received. All three values (start, end, step) must be of the same type.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, MessageType};
use async_trait::async_trait;
use futures::stream;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::{StreamExt, wrappers::ReceiverStream};

/// Configuration for RangeNode (currently unused, reserved for future extensibility).
pub struct RangeConfig {
  // Reserved for future configuration options
}

/// A node that generates number sequences (ranges) for iteration.
///
/// The node collects start, end, and step values from input ports and generates
/// a sequence of numbers, emitting each number as a separate output.
pub struct RangeNode {
  pub(crate) base: BaseNode,
  current_config: Arc<Mutex<Option<Arc<RangeConfig>>>>,
  // State to track received values
  start_value: Arc<Mutex<Option<Arc<dyn Any + Send + Sync>>>>,
  end_value: Arc<Mutex<Option<Arc<dyn Any + Send + Sync>>>>,
  step_value: Arc<Mutex<Option<Arc<dyn Any + Send + Sync>>>>,
}

impl RangeNode {
  /// Creates a new RangeNode with the given name.
  ///
  /// # Example
  ///
  /// ```rust,no_run
  /// use streamweave::nodes::RangeNode;
  ///
  /// let node = RangeNode::new("range".to_string());
  /// // Creates ports: configuration, start, end, step → out, error
  /// ```
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec![
          "configuration".to_string(),
          "start".to_string(),
          "end".to_string(),
          "step".to_string(),
        ],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
      start_value: Arc::new(Mutex::new(None)),
      end_value: Arc::new(Mutex::new(None)),
      step_value: Arc::new(Mutex::new(None)),
    }
  }

  /// Returns whether the node has a configuration set.
  pub fn has_config(&self) -> bool {
    self
      .current_config
      .try_lock()
      .map(|g| g.is_some())
      .unwrap_or(false)
  }
}

#[async_trait]
impl Node for RangeNode {
  fn name(&self) -> &str {
    self.base.name()
  }

  fn set_name(&mut self, name: &str) {
    self.base.set_name(name);
  }

  fn input_port_names(&self) -> &[String] {
    self.base.input_port_names()
  }

  fn output_port_names(&self) -> &[String] {
    self.base.output_port_names()
  }

  fn has_input_port(&self, name: &str) -> bool {
    self.base.has_input_port(name)
  }

  fn has_output_port(&self, name: &str) -> bool {
    self.base.has_output_port(name)
  }

  fn execute(
    &self,
    mut inputs: InputStreams,
  ) -> Pin<
    Box<dyn std::future::Future<Output = Result<OutputStreams, NodeExecutionError>> + Send + '_>,
  > {
    let config_state = Arc::clone(&self.current_config);
    let start_state = Arc::clone(&self.start_value);
    let end_state = Arc::clone(&self.end_value);
    let step_state = Arc::clone(&self.step_value);

    Box::pin(async move {
      // Extract input streams
      let config_stream = inputs
        .remove("configuration")
        .ok_or("Missing 'configuration' input")?;
      let start_stream = inputs.remove("start").ok_or("Missing 'start' input")?;
      let end_stream = inputs.remove("end").ok_or("Missing 'end' input")?;
      let step_stream = inputs.remove("step").ok_or("Missing 'step' input")?;

      // Tag streams to distinguish message types
      let config_stream =
        config_stream.map(|item| (MessageType::Config, "config".to_string(), item));
      let start_stream = start_stream.map(|item| (MessageType::Data, "start".to_string(), item));
      let end_stream = end_stream.map(|item| (MessageType::Data, "end".to_string(), item));
      let step_stream = step_stream.map(|item| (MessageType::Data, "step".to_string(), item));

      // Merge all streams
      let merged_stream = stream::select_all(vec![
        Box::pin(config_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(start_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(end_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
        Box::pin(step_stream)
          as Pin<
            Box<
              dyn tokio_stream::Stream<Item = (MessageType, String, Arc<dyn Any + Send + Sync>)>
                + Send,
            >,
          >,
      ]);

      // Create output channels
      let (out_tx, out_rx) = tokio::sync::mpsc::channel(10);
      let (error_tx, error_rx) = tokio::sync::mpsc::channel(10);

      // Process the merged stream
      let config_state_clone = Arc::clone(&config_state);
      let start_state_clone = Arc::clone(&start_state);
      let end_state_clone = Arc::clone(&end_state);
      let step_state_clone = Arc::clone(&step_state);
      let out_tx_clone = out_tx.clone();
      let error_tx_clone = error_tx.clone();

      tokio::spawn(async move {
        let mut merged = merged_stream;

        while let Some((msg_type, port_name, item)) = merged.next().await {
          match msg_type {
            MessageType::Config => {
              // Update configuration (currently unused, but store for future use)
              if let Ok(arc_config) = item.clone().downcast::<Arc<RangeConfig>>() {
                *config_state_clone.lock().await = Some(Arc::clone(&*arc_config));
              } else {
                // Configuration is optional, ignore invalid types
              }
            }
            MessageType::Data => {
              // Store the value based on port name
              match port_name.as_str() {
                "start" => {
                  *start_state_clone.lock().await = Some(item);
                }
                "end" => {
                  *end_state_clone.lock().await = Some(item);
                }
                "step" => {
                  *step_state_clone.lock().await = Some(item);
                }
                _ => {}
              }

              // Check if we have all three values
              let start_opt = start_state_clone.lock().await.clone();
              let end_opt = end_state_clone.lock().await.clone();
              let step_opt = step_state_clone.lock().await.clone();

              if let (Some(start), Some(end), Some(step)) = (start_opt, end_opt, step_opt) {
                // Try to generate range based on type
                let result = generate_range(start, end, step).await;

                match result {
                  Ok(numbers) => {
                    // Emit all numbers in the range
                    for num in numbers {
                      let _ = out_tx_clone.send(num).await;
                    }
                    // Clear the values after generating the range
                    *start_state_clone.lock().await = None;
                    *end_state_clone.lock().await = None;
                    *step_state_clone.lock().await = None;
                  }
                  Err(error_msg) => {
                    let error_arc: Arc<dyn Any + Send + Sync> = Arc::new(error_msg);
                    let _ = error_tx_clone.send(error_arc).await;
                    // Clear the values on error
                    *start_state_clone.lock().await = None;
                    *end_state_clone.lock().await = None;
                    *step_state_clone.lock().await = None;
                  }
                }
              }
            }
          }
        }
      });

      // Convert channels to streams
      let mut outputs = HashMap::new();
      outputs.insert(
        "out".to_string(),
        Box::pin(ReceiverStream::new(out_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );
      outputs.insert(
        "error".to_string(),
        Box::pin(ReceiverStream::new(error_rx))
          as Pin<Box<dyn tokio_stream::Stream<Item = Arc<dyn Any + Send + Sync>> + Send>>,
      );

      Ok(outputs)
    })
  }
}

/// Generates a range of numbers based on the provided start, end, and step values.
///
/// Supports i32, i64, u32, u64, f32, and f64 types.
async fn generate_range(
  start: Arc<dyn Any + Send + Sync>,
  end: Arc<dyn Any + Send + Sync>,
  step: Arc<dyn Any + Send + Sync>,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  // Try to downcast to different numeric types
  if let (Ok(start_i32), Ok(end_i32), Ok(step_i32)) = (
    start.clone().downcast::<i32>(),
    end.clone().downcast::<i32>(),
    step.clone().downcast::<i32>(),
  ) {
    return generate_i32_range(*start_i32, *end_i32, *step_i32);
  }

  if let (Ok(start_i64), Ok(end_i64), Ok(step_i64)) = (
    start.clone().downcast::<i64>(),
    end.clone().downcast::<i64>(),
    step.clone().downcast::<i64>(),
  ) {
    return generate_i64_range(*start_i64, *end_i64, *step_i64);
  }

  if let (Ok(start_u32), Ok(end_u32), Ok(step_u32)) = (
    start.clone().downcast::<u32>(),
    end.clone().downcast::<u32>(),
    step.clone().downcast::<u32>(),
  ) {
    return generate_u32_range(*start_u32, *end_u32, *step_u32);
  }

  if let (Ok(start_u64), Ok(end_u64), Ok(step_u64)) = (
    start.clone().downcast::<u64>(),
    end.clone().downcast::<u64>(),
    step.clone().downcast::<u64>(),
  ) {
    return generate_u64_range(*start_u64, *end_u64, *step_u64);
  }

  if let (Ok(start_f32), Ok(end_f32), Ok(step_f32)) = (
    start.clone().downcast::<f32>(),
    end.clone().downcast::<f32>(),
    step.clone().downcast::<f32>(),
  ) {
    return generate_f32_range(*start_f32, *end_f32, *step_f32);
  }

  if let (Ok(start_f64), Ok(end_f64), Ok(step_f64)) = (
    start.clone().downcast::<f64>(),
    end.clone().downcast::<f64>(),
    step.clone().downcast::<f64>(),
  ) {
    return generate_f64_range(*start_f64, *end_f64, *step_f64);
  }

  Err(
    "Unsupported type for range generation. Supported types: i32, i64, u32, u64, f32, f64"
      .to_string(),
  )
}

fn generate_i32_range(
  start: i32,
  end: i32,
  step: i32,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0 {
    return Err("Step size cannot be zero".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  if step > 0 {
    while current < end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current = current.saturating_add(step);
      if current <= start {
        // Overflow protection
        break;
      }
    }
  } else {
    while current > end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current = current.saturating_add(step);
      if current >= start {
        // Underflow protection
        break;
      }
    }
  }

  Ok(numbers)
}

fn generate_i64_range(
  start: i64,
  end: i64,
  step: i64,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0 {
    return Err("Step size cannot be zero".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  if step > 0 {
    while current < end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current = current.saturating_add(step);
      if current <= start {
        // Overflow protection
        break;
      }
    }
  } else {
    while current > end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current = current.saturating_add(step);
      if current >= start {
        // Underflow protection
        break;
      }
    }
  }

  Ok(numbers)
}

fn generate_u32_range(
  start: u32,
  end: u32,
  step: u32,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0 {
    return Err("Step size cannot be zero".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  while current < end {
    numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
    current = current.saturating_add(step);
    if current <= start {
      // Overflow protection
      break;
    }
  }

  Ok(numbers)
}

fn generate_u64_range(
  start: u64,
  end: u64,
  step: u64,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0 {
    return Err("Step size cannot be zero".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  while current < end {
    numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
    current = current.saturating_add(step);
    if current <= start {
      // Overflow protection
      break;
    }
  }

  Ok(numbers)
}

fn generate_f32_range(
  start: f32,
  end: f32,
  step: f32,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0.0 {
    return Err("Step size cannot be zero".to_string());
  }

  if step.is_nan() || start.is_nan() || end.is_nan() {
    return Err("NaN values are not supported for range generation".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  if step > 0.0 {
    while current < end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current += step;
      // Prevent infinite loops with floating point
      if numbers.len() > 1_000_000 {
        return Err("Range too large (over 1,000,000 elements)".to_string());
      }
    }
  } else {
    while current > end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current += step;
      // Prevent infinite loops with floating point
      if numbers.len() > 1_000_000 {
        return Err("Range too large (over 1,000,000 elements)".to_string());
      }
    }
  }

  Ok(numbers)
}

fn generate_f64_range(
  start: f64,
  end: f64,
  step: f64,
) -> Result<Vec<Arc<dyn Any + Send + Sync>>, String> {
  if step == 0.0 {
    return Err("Step size cannot be zero".to_string());
  }

  if step.is_nan() || start.is_nan() || end.is_nan() {
    return Err("NaN values are not supported for range generation".to_string());
  }

  let mut numbers = Vec::new();
  let mut current = start;

  if step > 0.0 {
    while current < end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current += step;
      // Prevent infinite loops with floating point
      if numbers.len() > 1_000_000 {
        return Err("Range too large (over 1,000,000 elements)".to_string());
      }
    }
  } else {
    while current > end {
      numbers.push(Arc::new(current) as Arc<dyn Any + Send + Sync>);
      current += step;
      // Prevent infinite loops with floating point
      if numbers.len() > 1_000_000 {
        return Err("Range too large (over 1,000,000 elements)".to_string());
      }
    }
  }

  Ok(numbers)
}

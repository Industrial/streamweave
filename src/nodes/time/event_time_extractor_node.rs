//! # Event Time Extractor Node
//!
//! A transform node that extracts event time from payloads and adds an `event_timestamp` field.
//! Use when the source provides payloads with event time (e.g. `HasEventTime`, Kafka timestamp,
//! or a configurable extractor). Downstream event-time window nodes can read `event_timestamp`.
//!
//! ## Ports
//!
//! - **Input**: `"configuration"` - Receives `EventTimeExtractorConfig` (extractor + optional fallback)
//! - **Input**: `"in"` - Receives data items
//! - **Output**: `"out"` - Sends items with `event_timestamp` (i64 ms) added
//! - **Output**: `"error"` - Sends errors
//!
//! ## Convention
//!
//! Output items have an `event_timestamp` field (i64 milliseconds). If the item is an object,
//! the field is added; otherwise the item is wrapped in `{ value, event_timestamp }`.

use crate::node::{InputStreams, Node, NodeExecutionError, OutputStreams};
use crate::nodes::common::{BaseNode, process_configurable_node};
use async_trait::async_trait;
use std::any::Any;
use std::collections::HashMap;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio_stream::wrappers::ReceiverStream;

/// Trait for extracting event time (ms) from a payload.
#[async_trait]
pub trait EventTimeExtractor: Send + Sync {
  /// Extracts event time in milliseconds (e.g. since Unix epoch), or `None` if not available.
  async fn event_time_ms(&self, value: Arc<dyn Any + Send + Sync>) -> Option<u64>;
}

/// Configuration for EventTimeExtractorNode. Wrapper to satisfy process_configurable_node's Clone bound.
#[derive(Clone)]
pub struct EventTimeExtractorConfigInner(Arc<dyn EventTimeExtractor>);

/// Configuration for EventTimeExtractorNode.
pub type EventTimeExtractorConfig = Arc<EventTimeExtractorConfigInner>;

/// Extractor that uses `HasEventTime` for payloads that implement it.
///
/// For `Arc<dyn Any>` payloads, attempts to downcast to common types that may implement
/// `HasEventTime`. Add impls as needed for your payload types.
struct HasEventTimeExtractor;

#[async_trait]
impl EventTimeExtractor for HasEventTimeExtractor {
  async fn event_time_ms(&self, value: Arc<dyn Any + Send + Sync>) -> Option<u64> {
    // Try downcast to HashMap with "timestamp" or "event_timestamp" field
    if let Ok(arc_map) = value
      .clone()
      .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
    {
      let map = arc_map.as_ref();
      if let Some(ts) = map.get("event_timestamp").or(map.get("timestamp")) {
        if let Ok(arc_i64) = ts.clone().downcast::<i64>() {
          return Some(*arc_i64 as u64);
        }
        if let Ok(arc_u64) = ts.clone().downcast::<u64>() {
          return Some(*arc_u64);
        }
      }
    }
    None
  }
}

/// Helper to create config that extracts from HashMap "event_timestamp" or "timestamp" fields.
pub fn event_time_from_map() -> EventTimeExtractorConfig {
  Arc::new(EventTimeExtractorConfigInner(Arc::new(
    HasEventTimeExtractor,
  )))
}

/// Wrapper for custom async extractor closures.
struct ClosureExtractor<F> {
  /// User-provided async closure that returns event time in ms.
  f: F,
}

#[async_trait]
impl<F, Fut> EventTimeExtractor for ClosureExtractor<F>
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync,
  Fut: std::future::Future<Output = Option<u64>> + Send,
{
  async fn event_time_ms(&self, value: Arc<dyn Any + Send + Sync>) -> Option<u64> {
    (self.f)(value).await
  }
}

/// Create config from an async closure.
pub fn event_time_extractor<F, Fut>(f: F) -> EventTimeExtractorConfig
where
  F: Fn(Arc<dyn Any + Send + Sync>) -> Fut + Send + Sync + 'static,
  Fut: std::future::Future<Output = Option<u64>> + Send + 'static,
{
  Arc::new(EventTimeExtractorConfigInner(Arc::new(ClosureExtractor {
    f,
  })))
}

/// Inserts or overwrites `event_timestamp` in a map payload, or wraps in `{ value, event_timestamp }`.
fn add_event_timestamp(
  item: Arc<dyn Any + Send + Sync>,
  event_time_ms: u64,
) -> Arc<dyn Any + Send + Sync> {
  let ts_arc = Arc::new(event_time_ms as i64) as Arc<dyn Any + Send + Sync>;
  if let Ok(arc_map) = item
    .clone()
    .downcast::<HashMap<String, Arc<dyn Any + Send + Sync>>>()
  {
    let mut new_map = (*arc_map).clone();
    new_map.insert("event_timestamp".to_string(), ts_arc);
    Arc::new(new_map) as Arc<dyn Any + Send + Sync>
  } else {
    let mut wrapped = HashMap::new();
    wrapped.insert("value".to_string(), item);
    wrapped.insert("event_timestamp".to_string(), ts_arc);
    Arc::new(wrapped) as Arc<dyn Any + Send + Sync>
  }
}

/// Node that extracts event time from payloads and adds `event_timestamp` field.
///
/// Configure with an [`EventTimeExtractor`] (e.g. [`event_time_from_map`] or
/// [`event_time_extractor`]). When extraction returns `None`, uses 0 (minimum).
pub struct EventTimeExtractorNode {
  /// Shared base node (ports, name).
  base: BaseNode,
  /// Currently active extractor config (reconfigurable at runtime).
  current_config: Arc<Mutex<Option<EventTimeExtractorConfig>>>,
}

impl EventTimeExtractorNode {
  /// Creates a new EventTimeExtractorNode.
  pub fn new(name: String) -> Self {
    Self {
      base: BaseNode::new(
        name,
        vec!["configuration".to_string(), "in".to_string()],
        vec!["out".to_string(), "error".to_string()],
      ),
      current_config: Arc::new(Mutex::new(None)),
    }
  }
}

#[async_trait]
impl Node for EventTimeExtractorNode {
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
    Box::pin(async move {
      let (out_rx, error_rx) = process_configurable_node(
        inputs
          .remove("configuration")
          .ok_or("Missing configuration")?,
        inputs.remove("in").ok_or("Missing 'in' input")?,
        config_state,
        |item, config| {
          let config = Arc::clone(config);
          Box::pin(async move {
            let ms = config.0.event_time_ms(Arc::clone(&item)).await.unwrap_or(0);
            Ok(Some(add_event_timestamp(item, ms)))
          })
        },
      );
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

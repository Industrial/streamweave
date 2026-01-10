//! XML parse transformer for StreamWeave
//!
//! Parses XML strings from stream items into JSON-like structures.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::collections::HashMap;
use std::pin::Pin;

/// A transformer that parses XML strings from stream items.
///
/// Input: String (XML string)
/// Output: `serde_json::Value` (parsed XML as JSON-like structure)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::XmlParseTransformer;
///
/// let transformer = XmlParseTransformer::new();
/// // Input: ["<root><name>Alice</name><age>30</age></root>"]
/// // Output: [Value::Object({"root": {"name": "Alice", "age": "30"}})]
/// ```
pub struct XmlParseTransformer {
  /// Transformer configuration
  config: TransformerConfig<String>,
}

impl XmlParseTransformer {
  /// Creates a new `XmlParseTransformer`.
  pub fn new() -> Self {
    Self {
      config: TransformerConfig::default(),
    }
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<String>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for XmlParseTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for XmlParseTransformer {
  fn clone(&self) -> Self {
    Self {
      config: self.config.clone(),
    }
  }
}

impl Input for XmlParseTransformer {
  type Input = String;
  type InputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

impl Output for XmlParseTransformer {
  type Output = Value;
  type OutputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

#[async_trait]
impl Transformer for XmlParseTransformer {
  type InputPorts = (String,);
  type OutputPorts = (Value,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let component_name = self
      .config
      .name
      .clone()
      .unwrap_or_else(|| "xml_parse_transformer".to_string());

    Box::pin(input.filter_map(move |item| {
      let component_name = component_name.clone();
      async move {
        match parse_xml_to_json(&item) {
          Ok(value) => Some(value),
          Err(e) => {
            tracing::warn!(
              component = %component_name,
              error = %e,
              "Failed to parse XML, skipping item"
            );
            None
          }
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<String>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<String> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<String> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<String>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<String>) -> ErrorContext<String> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "xml_parse_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

fn parse_xml_to_json(
  xml_str: &str,
) -> Result<Value, Box<dyn std::error::Error + Send + Sync + 'static>> {
  use quick_xml::Reader;
  use quick_xml::escape::unescape;
  use quick_xml::events::Event;

  let mut reader = Reader::from_str(xml_str);
  reader.config_mut().trim_text_start = true;
  reader.config_mut().trim_text_end = true;

  let mut stack: Vec<(String, HashMap<String, String>, Vec<Value>)> = Vec::new();
  let mut root: Option<Value> = None;

  loop {
    match reader.read_event() {
      Ok(Event::Start(e)) => {
        let name = String::from_utf8_lossy(e.name().as_ref()).to_string();
        let mut attrs = HashMap::new();
        for attr in e.attributes() {
          let attr = attr?;
          let key = String::from_utf8_lossy(attr.key.as_ref()).to_string();
          let value = String::from_utf8_lossy(&attr.value).to_string();
          attrs.insert(key, value);
        }
        stack.push((name, attrs, Vec::new()));
      }
      Ok(Event::Text(e)) => {
        if let Some((_, _, children)) = stack.last_mut() {
          let text_bytes = e.as_ref();
          let text_utf8 = String::from_utf8_lossy(text_bytes);
          let text = match unescape(text_utf8.as_ref()) {
            Ok(cow) => cow.into_owned(),
            Err(_) => text_utf8.into_owned(),
          };
          if !text.trim().is_empty() {
            children.push(Value::String(text.trim().to_string()));
          }
        }
      }
      Ok(Event::End(_)) => {
        if let Some((name, attrs, children)) = stack.pop() {
          let mut obj = serde_json::Map::new();

          // Add attributes if any
          if !attrs.is_empty() {
            let attrs_obj: serde_json::Map<String, Value> = attrs
              .into_iter()
              .map(|(k, v)| (k, Value::String(v)))
              .collect();
            obj.insert("@attributes".to_string(), Value::Object(attrs_obj));
          }

          // Add content/children
          if children.len() == 1 && children[0].is_string() {
            // Single text child - use as content
            obj.insert("content".to_string(), children[0].clone());
          } else if !children.is_empty() {
            // Multiple children or complex structure
            obj.insert("children".to_string(), Value::Array(children));
          }

          let value = Value::Object(obj);

          if stack.is_empty() {
            // Root element
            let mut root_obj = serde_json::Map::new();
            root_obj.insert(name, value);
            root = Some(Value::Object(root_obj));
          } else {
            // Add to parent's children
            if let Some((_, _, parent_children)) = stack.last_mut() {
              let mut elem_obj = serde_json::Map::new();
              elem_obj.insert(name, value);
              parent_children.push(Value::Object(elem_obj));
            }
          }
        }
      }
      Ok(Event::Eof) => break,
      Err(e) => return Err(Box::new(e)),
      _ => {}
    }
  }

  root.ok_or_else(|| "No root element found".into())
}

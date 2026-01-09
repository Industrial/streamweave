//! XML stringify transformer for StreamWeave
//!
//! Converts JSON values from stream items into XML strings.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::{Stream, StreamExt};
use serde_json::Value;
use std::pin::Pin;

/// Configuration for XML stringification.
#[derive(Debug, Clone, Default)]
pub struct XmlStringifyConfig {
  /// Whether to pretty-print XML.
  pub pretty: bool,
  /// Root element name (if input is not an object).
  pub root_element: Option<String>,
}

/// A transformer that stringifies JSON values to XML from stream items.
///
/// Input: `serde_json::Value` (JSON object or value)
/// Output: String (XML string)
///
/// # Example
///
/// ```rust
/// use streamweave::transformers::XmlStringifyTransformer;
/// use serde_json::json;
///
/// let transformer = XmlStringifyTransformer::new();
/// // Input: [json!({"root": {"name": "Alice", "age": 30}})]
/// // Output: ["<root><name>Alice</name><age>30</age></root>"]
/// ```
pub struct XmlStringifyTransformer {
  /// XML stringification configuration
  xml_config: XmlStringifyConfig,
  /// Transformer configuration
  config: TransformerConfig<Value>,
}

impl XmlStringifyTransformer {
  /// Creates a new `XmlStringifyTransformer`.
  pub fn new() -> Self {
    Self {
      xml_config: XmlStringifyConfig::default(),
      config: TransformerConfig::default(),
    }
  }

  /// Sets whether to pretty-print XML.
  pub fn with_pretty(mut self, pretty: bool) -> Self {
    self.xml_config.pretty = pretty;
    self
  }

  /// Sets the root element name.
  pub fn with_root_element(mut self, root: impl Into<String>) -> Self {
    self.xml_config.root_element = Some(root.into());
    self
  }

  /// Sets the error handling strategy for this transformer.
  pub fn with_error_strategy(mut self, strategy: ErrorStrategy<Value>) -> Self {
    self.config.error_strategy = strategy;
    self
  }

  /// Sets the name for this transformer.
  pub fn with_name(mut self, name: String) -> Self {
    self.config.name = Some(name);
    self
  }
}

impl Default for XmlStringifyTransformer {
  fn default() -> Self {
    Self::new()
  }
}

impl Clone for XmlStringifyTransformer {
  fn clone(&self) -> Self {
    Self {
      xml_config: self.xml_config.clone(),
      config: self.config.clone(),
    }
  }
}

impl Input for XmlStringifyTransformer {
  type Input = Value;
  type InputStream = Pin<Box<dyn Stream<Item = Value> + Send>>;
}

impl Output for XmlStringifyTransformer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = String> + Send>>;
}

#[async_trait]
impl Transformer for XmlStringifyTransformer {
  type InputPorts = (Value,);
  type OutputPorts = (String,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let xml_config = self.xml_config.clone();

    Box::pin(input.filter_map(move |item| {
      let xml_config = xml_config.clone();
      async move { json_to_xml(&item, &xml_config).ok() }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Value>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Value> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Value> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Value>) -> ErrorAction {
    match self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Value>) -> ErrorContext<Value> {
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
        .unwrap_or_else(|| "xml_stringify_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

fn json_to_xml(
  value: &Value,
  config: &XmlStringifyConfig,
) -> Result<String, Box<dyn std::error::Error + Send + Sync + 'static>> {
  use quick_xml::Writer;
  use quick_xml::events::{BytesDecl, Event};

  let mut writer = Writer::new(Vec::new());
  writer.write_event(Event::Decl(BytesDecl::new("1.0", Some("UTF-8"), None)))?;

  match value {
    Value::Object(obj) => {
      // If object has single key, use it as root
      if obj.len() == 1 {
        let (root_name, root_value) = obj.iter().next().unwrap();
        value_to_xml(root_value, root_name, &mut writer, config)?;
      } else {
        let root_name = config.root_element.as_deref().unwrap_or("root");
        value_to_xml(value, root_name, &mut writer, config)?;
      }
    }
    _ => {
      let root_name = config.root_element.as_deref().unwrap_or("root");
      value_to_xml(value, root_name, &mut writer, config)?;
    }
  }

  let xml = writer.into_inner();
  Ok(String::from_utf8(xml)?)
}

#[allow(clippy::only_used_in_recursion)]
fn value_to_xml(
  value: &Value,
  element_name: &str,
  writer: &mut quick_xml::Writer<Vec<u8>>,
  config: &XmlStringifyConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
  use quick_xml::events::{BytesEnd, BytesStart, Event};

  match value {
    Value::Object(obj) => {
      let mut start = BytesStart::new(element_name);

      // Handle @attributes
      if let Some(Value::Object(attrs)) = obj.get("@attributes") {
        for (k, v) in attrs {
          if let Value::String(v_str) = v {
            start.push_attribute((k.as_str(), v_str.as_str()));
          }
        }
      }

      writer.write_event(Event::Start(start))?;

      // Write children (excluding @attributes, content, children)
      for (k, v) in obj {
        match k.as_str() {
          "@attributes" => {} // Already handled
          "content" => {
            // Content is text value
            if let Value::String(s) = v {
              writer.write_event(Event::Text(quick_xml::events::BytesText::from_escaped(
                s.as_str(),
              )))?;
            }
          }
          "children" => {
            // Children array - recurse with same element name
            if let Value::Array(arr) = v {
              for item in arr {
                value_to_xml(item, element_name, writer, config)?;
              }
            }
          }
          _ => {
            // Regular child element
            value_to_xml(v, k, writer, config)?;
          }
        }
      }

      writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    }
    Value::Array(arr) => {
      for item in arr {
        value_to_xml(item, element_name, writer, config)?;
      }
    }
    Value::String(s) => {
      let start = BytesStart::new(element_name);
      writer.write_event(Event::Start(start))?;
      writer.write_event(Event::Text(quick_xml::events::BytesText::from_escaped(
        s.as_str(),
      )))?;
      writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    }
    Value::Number(n) => {
      let start = BytesStart::new(element_name);
      writer.write_event(Event::Start(start))?;
      writer.write_event(Event::Text(quick_xml::events::BytesText::from_escaped(
        n.to_string().as_str(),
      )))?;
      writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    }
    Value::Bool(b) => {
      let start = BytesStart::new(element_name);
      writer.write_event(Event::Start(start))?;
      writer.write_event(Event::Text(quick_xml::events::BytesText::from_escaped(
        b.to_string().as_str(),
      )))?;
      writer.write_event(Event::End(BytesEnd::new(element_name)))?;
    }
    Value::Null => {
      let start = BytesStart::new(element_name);
      writer.write_event(Event::Empty(start))?;
    }
  }

  Ok(())
}

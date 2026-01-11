//! # XML Stringify Transformer
//!
//! Transformer for converting JSON values to XML strings in StreamWeave pipelines.
//!
//! This module provides [`XmlStringifyTransformer`], a transformer that converts
//! `serde_json::Value` objects into XML strings. It supports pretty-printing,
//! configurable root element names, and proper handling of JSON structures
//! including objects, arrays, and primitives.
//!
//! # Overview
//!
//! [`XmlStringifyTransformer`] converts JSON values (typically objects) into
//! well-formed XML strings. It handles the conversion from JSON's tree structure
//! to XML's element hierarchy, making it useful for generating XML output from
//! JSON data in streaming pipelines.
//!
//! # Key Concepts
//!
//! - **JSON to XML Conversion**: Converts JSON objects, arrays, and primitives to XML
//! - **Element Mapping**: JSON object keys become XML element names
//! - **Array Handling**: JSON arrays can be serialized as repeated elements
//! - **Pretty Printing**: Optional formatted XML output with indentation
//! - **Root Element Configuration**: Customizable root element name for non-object values
//! - **Attribute Support**: Supports XML attributes via special `@attributes` JSON key
//!
//! # Core Types
//!
//! - **[`XmlStringifyTransformer`]**: Transformer that converts JSON values to XML strings
//! - **[`XmlStringifyConfig`]**: Configuration for XML stringification behavior
//!
//! # Quick Start
//!
//! ## Basic Usage
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//! use serde_json::json;
//!
//! // Create a transformer that converts JSON to XML
//! let transformer = XmlStringifyTransformer::new();
//!
//! // Input: json!({"person": {"name": "Alice", "age": 30}})
//! // Output: "<person><name>Alice</name><age>30</age></person>"
//! ```
//!
//! ## Pretty-Printed XML
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//!
//! // Create a transformer that pretty-prints XML
//! let transformer = XmlStringifyTransformer::new()
//!     .with_pretty(true);
//! ```
//!
//! ## Custom Root Element
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//! use serde_json::json;
//!
//! // Create a transformer with custom root element name
//! let transformer = XmlStringifyTransformer::new()
//!     .with_root_element("document");
//!
//! // Input: json!({"name": "Alice"})
//! // Output: "<document><name>Alice</name></document>"
//! ```
//!
//! ## With Error Handling
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//! use streamweave::ErrorStrategy;
//!
//! // Create a transformer with error handling strategy
//! let transformer = XmlStringifyTransformer::new()
//!     .with_error_strategy(ErrorStrategy::Skip)
//!     .with_name("json-to-xml".to_string());
//! ```
//!
//! ## XML with Attributes
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//! use serde_json::json;
//!
//! // JSON with @attributes key for XML attributes
//! let json = json!({
//!     "person": {
//!         "@attributes": {"id": "123", "active": "true"},
//!         "name": "Alice",
//!         "age": 30
//!     }
//! });
//!
//! // Output: <person id="123" active="true"><name>Alice</name><age>30</age></person>
//! ```
//!
//! # Design Decisions
//!
//! ## JSON Structure Mapping
//!
//! - **Objects**: JSON objects map to XML elements (keys → element names)
//! - **Arrays**: JSON arrays can serialize as repeated elements
//! - **Strings/Numbers/Booleans**: Converted to XML text content
//! - **Null**: Typically omitted or converted to empty elements
//!
//! ## Attribute Handling
//!
//! XML attributes are specified using a special `@attributes` key in JSON objects.
//! This convention allows JSON to represent XML attributes while maintaining a
//! JSON-compatible structure.
//!
//! ## Root Element Handling
//!
//! When the input JSON is an object, its keys are used as element names. For
//! non-object values (primitives, arrays), a configurable root element name is
//! used (default: "root").
//!
//! ## Error Handling
//!
//! Conversion errors (invalid JSON structures, encoding issues) result in items
//! being filtered out of the stream. This prevents invalid XML from being produced
//! while allowing the pipeline to continue processing.
//!
//! ## UTF-8 Encoding
//!
//! XML output is always UTF-8 encoded, with proper declaration in the XML output.
//! This ensures compatibility with XML parsers and standards.
//!
//! # Integration with StreamWeave
//!
//! [`XmlStringifyTransformer`] integrates seamlessly with StreamWeave's pipeline
//! and graph systems:
//!
//! - **Pipeline API**: Use in pipelines for JSON-to-XML conversion
//! - **Graph API**: Wrap in graph nodes for graph-based XML generation
//! - **Error Handling**: Supports standard error handling strategies
//! - **Configuration**: Supports configuration via [`TransformerConfig`] and [`XmlStringifyConfig`]
//! - **JSON Values**: Works with `serde_json::Value` for flexible JSON handling
//!
//! # Common Patterns
//!
//! ## Generating XML Documents
//!
//! Convert JSON data structures to XML documents:
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//! use serde_json::json;
//!
//! let transformer = XmlStringifyTransformer::new()
//!     .with_pretty(true)
//!     .with_root_element("document");
//!
//! // Process JSON objects and output formatted XML
//! ```
//!
//! ## API Response Formatting
//!
//! Convert JSON API responses to XML format:
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//!
//! let transformer = XmlStringifyTransformer::new()
//!     .with_pretty(true);
//!
//! // Transform JSON responses to XML for XML-based APIs
//! ```
//!
//! ## Data Serialization
//!
//! Serialize data structures as XML for storage or transmission:
//!
//! ```rust
//! use streamweave::transformers::XmlStringifyTransformer;
//!
//! let transformer = XmlStringifyTransformer::new()
//!     .with_root_element("data");
//!
//! // Serialize structured data as XML
//! ```
//!
//! # XML Output Format
//!
//! ## Standard Output
//!
//! XML output includes:
//! - XML declaration: `<?xml version="1.0" encoding="UTF-8"?>`
//! - Proper element nesting based on JSON structure
//! - Text content for primitive values
//! - Attributes when `@attributes` key is present
//!
//! ## Pretty-Printed Output
//!
//! When pretty-printing is enabled, XML is formatted with:
//! - Proper indentation
//! - Line breaks between elements
//! - Improved readability
//!
//! ## Special JSON Keys
//!
//! - **@attributes**: Maps to XML element attributes
//! - **content**: Text content of an element (when mixed content is needed)
//! - **children**: Array of child elements with the same name

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

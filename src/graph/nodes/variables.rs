//! Graph-level variable store and transformers for reading/writing variables.
//!
//! Provides thread-safe access to variables that can be shared between nodes
//! in the graph.

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, StreamError};
use crate::{Input, Output, Transformer, TransformerConfig};
use async_trait::async_trait;
use futures::Stream;
use futures::StreamExt;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

/// Graph-level variable store for sharing state between nodes.
///
/// Provides thread-safe access to variables that can be read and written
/// by different nodes in the graph. Variables are stored as type-erased
/// `Box<dyn Any + Send + Sync>` and must be downcast when retrieved.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::GraphVariables;
///
/// let vars = GraphVariables::new();
/// vars.set("counter", 42i32);
/// let value: Option<i32> = vars.get("counter");
/// assert_eq!(value, Some(42));
/// ```
#[derive(Clone)]
pub struct GraphVariables {
  /// Map of variable names to type-erased values
  pub(crate) variables: Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
}

impl GraphVariables {
  /// Creates a new `GraphVariables` instance.
  ///
  /// # Returns
  ///
  /// A new empty variable store.
  pub fn new() -> Self {
    Self {
      variables: Arc::new(RwLock::new(HashMap::new())),
    }
  }

  /// Sets a variable value.
  ///
  /// # Arguments
  ///
  /// * `name` - Variable name
  /// * `value` - Value to store (must be `Send + Sync + 'static`)
  pub fn set<T: Send + Sync + 'static>(&self, name: &str, value: T) {
    let mut vars = self.variables.write().unwrap();
    vars.insert(name.to_string(), Box::new(value));
  }

  /// Gets a variable value.
  ///
  /// # Arguments
  ///
  /// * `name` - Variable name
  ///
  /// # Returns
  ///
  /// `Some(value)` if the variable exists and can be downcast to `T`,
  /// `None` otherwise.
  pub fn get<T>(&self, name: &str) -> Option<T>
  where
    T: Clone + Send + Sync + 'static,
  {
    let vars = self.variables.read().unwrap();
    vars.get(name).and_then(|v| v.downcast_ref::<T>()).cloned()
  }

  /// Gets a variable value, or sets it with a factory function if it doesn't exist.
  ///
  /// # Arguments
  ///
  /// * `name` - Variable name
  /// * `factory` - Function that produces the initial value if the variable doesn't exist
  ///
  /// # Returns
  ///
  /// The value of the variable (either existing or newly created).
  pub fn get_or_set<T, F>(&self, name: &str, factory: F) -> T
  where
    T: Clone + Send + Sync + 'static,
    F: FnOnce() -> T,
  {
    {
      let vars = self.variables.read().unwrap();
      if let Some(value) = vars.get(name).and_then(|v| v.downcast_ref::<T>()) {
        return value.clone();
      }
    }
    let mut vars = self.variables.write().unwrap();
    let value = factory();
    vars.insert(name.to_string(), Box::new(value.clone()));
    value
  }

  /// Removes a variable.
  ///
  /// # Arguments
  ///
  /// * `name` - Variable name
  ///
  /// # Returns
  ///
  /// `true` if the variable was removed, `false` if it didn't exist.
  pub fn remove(&self, name: &str) -> bool {
    let mut vars = self.variables.write().unwrap();
    vars.remove(name).is_some()
  }
}

impl Default for GraphVariables {
  fn default() -> Self {
    Self::new()
  }
}

/// Transformer that reads a variable value and emits it for each input item.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{GraphVariables, ReadVariable};
/// use crate::graph::node::TransformerNode;
///
/// let vars = GraphVariables::new();
/// vars.set("counter", 42i32);
///
/// let read_var = ReadVariable::new("counter", vars);
/// let node = TransformerNode::from_transformer(
///     "read_counter".to_string(),
///     read_var,
/// );
/// ```
pub struct ReadVariable<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Variable name to read
  variable_name: String,
  /// Reference to graph variables
  variables: Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
  /// Configuration for the transformer
  config: TransformerConfig<()>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> ReadVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `ReadVariable` transformer.
  ///
  /// # Arguments
  ///
  /// * `variable_name` - Name of the variable to read
  /// * `variables` - Reference to the graph variables store
  ///
  /// # Returns
  ///
  /// A new `ReadVariable` transformer instance.
  pub fn new(variable_name: String, variables: GraphVariables) -> Self {
    Self {
      variable_name,
      variables: Arc::clone(&variables.variables),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for ReadVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = ();
  type InputStream = Pin<Box<dyn Stream<Item = ()> + Send>>;
}

impl<T> Output for ReadVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for ReadVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = ((),);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let variable_name = self.variable_name.clone();
    let variables = Arc::clone(&self.variables);
    Box::pin(input.filter_map(move |_| {
      let variable_name = variable_name.clone();
      let variables = Arc::clone(&variables);
      async move {
        let vars = variables.read().unwrap();
        vars
          .get(&variable_name)
          .and_then(|v| v.downcast_ref::<T>())
          .cloned()
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "read_variable".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

/// Transformer that writes input items to a variable.
///
/// # Example
///
/// ```rust
/// use crate::graph::control_flow::{GraphVariables, WriteVariable};
/// use crate::graph::node::TransformerNode;
///
/// let vars = GraphVariables::new();
/// let write_var = WriteVariable::new("counter", vars);
/// let node = TransformerNode::from_transformer(
///     "write_counter".to_string(),
///     write_var,
/// );
/// ```
pub struct WriteVariable<T: std::fmt::Debug + Clone + Send + Sync + 'static> {
  /// Variable name to write
  variable_name: String,
  /// Reference to graph variables
  variables: Arc<RwLock<HashMap<String, Box<dyn std::any::Any + Send + Sync>>>>,
  /// Configuration for the transformer
  config: TransformerConfig<T>,
  /// Phantom data for type parameter
  _phantom: PhantomData<T>,
}

impl<T> WriteVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  /// Creates a new `WriteVariable` transformer.
  ///
  /// # Arguments
  ///
  /// * `variable_name` - Name of the variable to write
  /// * `variables` - Reference to the graph variables store
  ///
  /// # Returns
  ///
  /// A new `WriteVariable` transformer instance.
  pub fn new(variable_name: String, variables: GraphVariables) -> Self {
    Self {
      variable_name,
      variables: Arc::clone(&variables.variables),
      config: TransformerConfig::default(),
      _phantom: PhantomData,
    }
  }
}

impl<T> Input for WriteVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Input = T;
  type InputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

impl<T> Output for WriteVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type Output = T;
  type OutputStream = Pin<Box<dyn Stream<Item = T> + Send>>;
}

#[async_trait]
impl<T> Transformer for WriteVariable<T>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
{
  type InputPorts = (T,);
  type OutputPorts = (T,);

  async fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let variable_name = self.variable_name.clone();
    let variables = Arc::clone(&self.variables);
    Box::pin(input.map(move |item| {
      let mut vars = variables.write().unwrap();
      vars.insert(variable_name.clone(), Box::new(item.clone()));
      item
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<Self::Input>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<Self::Input> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<Self::Input> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<Self::Input>) -> ErrorAction {
    match self.config.error_strategy() {
      crate::error::ErrorStrategy::Stop => ErrorAction::Stop,
      crate::error::ErrorStrategy::Skip => ErrorAction::Skip,
      crate::error::ErrorStrategy::Retry(n) if error.retries < n => ErrorAction::Retry,
      crate::error::ErrorStrategy::Custom(ref handler) => handler(error),
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<Self::Input>) -> ErrorContext<Self::Input> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: self.component_info().type_name,
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name()
        .clone()
        .unwrap_or_else(|| "write_variable".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

use futures::StreamExt;
use proptest::prelude::*;
use streamweave::{Producer, Transformer};
use streamweave_graph::control_flow::{GraphVariables, ReadVariable, WriteVariable};
use streamweave_vec::VecProducer;

#[test]
fn test_graph_variables_new() {
  let vars = GraphVariables::new();
  // Just verify it can be created
  assert!(vars.get::<i32>("key").is_none());
}

#[test]
fn test_graph_variables_set_get() {
  let vars = GraphVariables::new();
  vars.set("key1", 42i32);
  vars.set("key2", "value".to_string());

  assert_eq!(vars.get::<i32>("key1"), Some(42));
  assert_eq!(vars.get::<String>("key2"), Some("value".to_string()));
  assert!(vars.get::<i32>("key2").is_none()); // Wrong type
  assert!(vars.get::<String>("key1").is_none()); // Wrong type
}

#[test]
fn test_graph_variables_get_or_set() {
  let vars = GraphVariables::new();
  let value1 = vars.get_or_set("key", || 10i32);
  assert_eq!(value1, 10);

  let value2 = vars.get_or_set("key", || 20i32);
  assert_eq!(value2, 10); // Should return existing value
}

#[test]
fn test_graph_variables_remove() {
  let vars = GraphVariables::new();
  vars.set("counter", 42i32);

  assert!(vars.remove("counter"));
  assert_eq!(vars.get::<i32>("counter"), None);
  assert!(!vars.remove("nonexistent"));
}

#[test]
fn test_graph_variables_default() {
  let vars1 = GraphVariables::new();
  let vars2 = GraphVariables::default();
  assert_eq!(vars1.get::<i32>("test"), vars2.get::<i32>("test"));
}

#[tokio::test]
async fn test_read_variable_transformer() {
  let vars = GraphVariables::new();
  vars.set("counter", 42i32);

  let mut read_var = ReadVariable::<i32>::new("counter".to_string(), vars);
  let mut producer = VecProducer::new(vec![(), (), ()]);
  let stream = producer.produce();

  let mut output = read_var.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  assert_eq!(results, vec![42, 42, 42]);
}

#[tokio::test]
async fn test_read_variable_nonexistent() {
  let vars = GraphVariables::new();
  let mut read_var = ReadVariable::<i32>::new("nonexistent".to_string(), vars);
  let mut producer = VecProducer::new(vec![(), ()]);
  let stream = producer.produce();

  let mut output = read_var.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should filter out items when variable doesn't exist
  assert_eq!(results.len(), 0);
}

#[tokio::test]
async fn test_write_variable_transformer() {
  let vars = GraphVariables::new();
  let vars_ref = vars.clone();
  let mut write_var = WriteVariable::new("counter".to_string(), vars);
  let mut producer = VecProducer::new(vec![1, 2, 3]);
  let stream = producer.produce();

  let mut output = write_var.transform(Box::pin(stream)).await;
  let mut results = Vec::new();
  while let Some(item) = output.next().await {
    results.push(item);
  }

  // Should pass through items
  assert_eq!(results, vec![1, 2, 3]);

  // Variable should be set to last value
  assert_eq!(vars_ref.get::<i32>("counter"), Some(3));
}

proptest! {
  #[test]
  fn test_graph_variables_proptest(
    key in "[a-z]{1,10}",
    value in -1000i32..1000
  ) {
    let vars = GraphVariables::new();
    vars.set(&key, value);
    assert_eq!(vars.get::<i32>(&key), Some(value));
  }
}

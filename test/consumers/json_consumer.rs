use serde::{Deserialize, Serialize};
use streamweave::consumers::JsonConsumer;
use streamweave::pipeline::PipelineBuilder;
use streamweave::prelude::*;
use streamweave::producers::ArrayProducer;
use tempfile::NamedTempFile;
use tokio::fs::read_to_string;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
struct TestData {
  id: u32,
  name: String,
}

#[tokio::test]
async fn test_json_consumer_array() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let data = vec![
    TestData {
      id: 1,
      name: "Alice".to_string(),
    },
    TestData {
      id: 2,
      name: "Bob".to_string(),
    },
  ];

  let producer = ArrayProducer::new(data.clone());
  let mut consumer = JsonConsumer::<TestData>::new(&path).with_as_array(true);

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);
  pipeline.run().await.unwrap();

  // Read and verify
  let content = read_to_string(&path).await.unwrap();
  let parsed: Vec<TestData> = serde_json::from_str(&content).unwrap();
  assert_eq!(parsed.len(), 2);
  assert_eq!(parsed[0].id, 1);
  assert_eq!(parsed[1].id, 2);
}

#[tokio::test]
async fn test_json_consumer_pretty() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let data = vec![TestData {
    id: 1,
    name: "Alice".to_string(),
  }];

  let producer = ArrayProducer::new(data);
  let mut consumer = JsonConsumer::<TestData>::new(&path)
    .with_as_array(true)
    .with_pretty(true);

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);
  pipeline.run().await.unwrap();

  // Read and verify it's pretty-printed (contains newlines)
  let content = read_to_string(&path).await.unwrap();
  assert!(content.contains('\n'));
  let parsed: Vec<TestData> = serde_json::from_str(&content).unwrap();
  assert_eq!(parsed.len(), 1);
}

#[tokio::test]
async fn test_json_consumer_single_value() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_path_buf();

  let data = TestData {
    id: 1,
    name: "Alice".to_string(),
  };

  let producer = ArrayProducer::new([data.clone()]);
  let mut consumer = JsonConsumer::<TestData>::new(&path).with_as_array(false);

  let pipeline = PipelineBuilder::new().producer(producer).consumer(consumer);
  pipeline.run().await.unwrap();

  // Read and verify it's a single object, not an array
  let content = read_to_string(&path).await.unwrap();
  let parsed: TestData = serde_json::from_str(&content).unwrap();
  assert_eq!(parsed.id, 1);
  assert_eq!(parsed.name, "Alice");
  // Verify it's not an array (should not start with '[')
  assert!(!content.trim_start().starts_with('['));
}

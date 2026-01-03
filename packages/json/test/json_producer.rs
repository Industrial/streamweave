use futures::StreamExt;
use serde::{Deserialize, Serialize};
use streamweave::error::ErrorStrategy;
use streamweave_json::JsonProducer;
use tempfile::NamedTempFile;
use tokio::fs::write;

#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]
struct TestData {
  id: u32,
  name: String,
}

#[tokio::test]
async fn test_json_producer_array_as_stream() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write a JSON array
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
  let json = serde_json::to_string(&data).unwrap();
  write(&path, json).await.unwrap();

  // Read and stream each element
  let mut producer = JsonProducer::<TestData>::new(&path).with_array_as_stream(true);
  let mut stream = producer.produce();

  let mut items = Vec::new();
  while let Some(item) = stream.next().await {
    items.push(item);
  }

  assert_eq!(items.len(), 2);
  assert_eq!(items[0].id, 1);
  assert_eq!(items[1].id, 2);
}

#[tokio::test]
async fn test_json_producer_array_as_single_item() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write a JSON array
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
  let json = serde_json::to_string(&data).unwrap();
  write(&path, json).await.unwrap();

  // Read the entire array as a single item
  let mut producer = JsonProducer::<Vec<TestData>>::new(&path).with_array_as_stream(false);
  let mut stream = producer.produce();

  let item = stream.next().await.unwrap();
  assert_eq!(item.len(), 2);
  assert_eq!(item[0].id, 1);
  assert_eq!(item[1].id, 2);
}

#[tokio::test]
async fn test_json_producer_single_object() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write a single JSON object
  let data = TestData {
    id: 1,
    name: "Alice".to_string(),
  };
  let json = serde_json::to_string(&data).unwrap();
  write(&path, json).await.unwrap();

  // Read the object
  let mut producer = JsonProducer::<TestData>::new(&path);
  let mut stream = producer.produce();

  let item = stream.next().await.unwrap();
  assert_eq!(item.id, 1);
  assert_eq!(item.name, "Alice");
}

#[tokio::test]
async fn test_json_producer_error_handling() {
  let file = NamedTempFile::new().unwrap();
  let path = file.path().to_str().unwrap().to_string();

  // Write invalid JSON
  write(&path, "invalid json").await.unwrap();

  // Should handle error gracefully with Skip strategy
  let mut producer = JsonProducer::<TestData>::new(&path).with_error_strategy(ErrorStrategy::Skip);
  let mut stream = producer.produce();

  // Should produce empty stream
  assert!(stream.next().await.is_none());
}

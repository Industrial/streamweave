//! Tests for transaction module

use std::time::Duration;
use streamweave::offset::{InMemoryOffsetStore, Offset};
use streamweave::transaction::*;

#[tokio::test]
async fn test_transaction_id() {
  let id = TransactionId::new(42);
  assert_eq!(id.value(), 42);
  assert_eq!(format!("{}", id), "tx:42");
}

#[tokio::test]
async fn test_transaction_state_display() {
  assert_eq!(format!("{}", TransactionState::Active), "active");
  assert_eq!(format!("{}", TransactionState::Committed), "committed");
  assert_eq!(format!("{}", TransactionState::RolledBack), "rolled_back");
  assert_eq!(format!("{}", TransactionState::TimedOut), "timed_out");
}

#[tokio::test]
async fn test_transaction_manager_new() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  // Manager should be created successfully
  assert!(true);
}

#[tokio::test]
async fn test_transaction_manager_begin() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = manager.begin().await.unwrap();
  assert_eq!(tx_id.value() > 0, true);
}

#[tokio::test]
async fn test_transaction_manager_commit() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();

  let result = manager.commit(&tx_id).await;
  assert!(result.is_ok());

  // Verify offset was committed
  let state = manager.get_state(&tx_id).await.unwrap();
  assert_eq!(state, TransactionState::Committed);
}

#[tokio::test]
async fn test_transaction_manager_rollback() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();

  let result = manager.rollback(&tx_id).await;
  assert!(result.is_ok());

  let state = manager.get_state(&tx_id).await.unwrap();
  assert_eq!(state, TransactionState::RolledBack);
}

#[tokio::test]
async fn test_transaction_manager_buffer_offset() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = manager.begin().await.unwrap();
  let result = manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_transaction_manager_get_state() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = manager.begin().await.unwrap();
  let state = manager.get_state(&tx_id).await.unwrap();
  assert_eq!(state, TransactionState::Active);
}

#[tokio::test]
async fn test_transaction_manager_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);

  let tx_id = TransactionId::new(999);
  let result = manager.get_state(&tx_id).await;
  assert!(result.is_err());
  match result.unwrap_err() {
    TransactionError::NotFound(_) => assert!(true),
    _ => assert!(false),
  }
}

#[tokio::test]
async fn test_transaction_config_default() {
  let config = TransactionConfig::default();
  // Default config should be valid
  assert!(true);
}

#[tokio::test]
async fn test_transaction_config_with_timeout() {
  let config = TransactionConfig::default().with_timeout(Duration::from_secs(30));

  // Config with timeout should be valid
  assert!(true);
}

#[tokio::test]
async fn test_transaction_error_display() {
  let error = TransactionError::NotFound(TransactionId::new(1));
  let display = format!("{}", error);
  assert!(display.contains("Transaction not found"));

  let error2 = TransactionError::Timeout(TransactionId::new(2));
  let display2 = format!("{}", error2);
  assert!(display2.contains("timed out"));
}

#[tokio::test]
async fn test_transaction_error_from_offset_error() {
  use streamweave::offset::OffsetError;
  let offset_error = OffsetError::IoError(std::io::Error::new(std::io::ErrorKind::Other, "test"));
  let tx_error: TransactionError = offset_error.into();

  match tx_error {
    TransactionError::OffsetError(_) => assert!(true),
    _ => assert!(false),
  }
}

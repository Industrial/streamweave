use crate::offset::{InMemoryOffsetStore, Offset, OffsetError};
use crate::transaction::*;
use std::time::Duration;
use tokio::time::sleep;

// ============================================================================
// TransactionId Tests
// ============================================================================

#[test]
fn test_transaction_id_new() {
  let id = TransactionId::new(42);
  assert_eq!(id.value(), 42);
}

#[test]
fn test_transaction_id_value() {
  let id = TransactionId::new(100);
  assert_eq!(id.value(), 100);
}

#[test]
fn test_transaction_id_clone() {
  let id1 = TransactionId::new(42);
  let id2 = id1;
  assert_eq!(id1, id2);
}

#[test]
fn test_transaction_id_eq() {
  assert_eq!(TransactionId::new(1), TransactionId::new(1));
  assert_ne!(TransactionId::new(1), TransactionId::new(2));
}

#[test]
fn test_transaction_id_hash() {
  use std::collections::HashSet;
  let mut set = HashSet::new();
  set.insert(TransactionId::new(1));
  set.insert(TransactionId::new(2));
  assert_eq!(set.len(), 2);
  assert!(set.contains(&TransactionId::new(1)));
}

#[test]
fn test_transaction_id_display() {
  let id = TransactionId::new(42);
  assert_eq!(format!("{}", id), "tx:42");
}

#[test]
fn test_transaction_id_debug() {
  let id = TransactionId::new(42);
  let _ = format!("{:?}", id);
}

// ============================================================================
// TransactionState Tests
// ============================================================================

#[test]
fn test_transaction_state_variants() {
  assert_eq!(TransactionState::Active, TransactionState::Active);
  assert_eq!(TransactionState::Committed, TransactionState::Committed);
  assert_eq!(TransactionState::RolledBack, TransactionState::RolledBack);
  assert_eq!(TransactionState::TimedOut, TransactionState::TimedOut);
}

#[test]
fn test_transaction_state_clone() {
  let state = TransactionState::Active;
  assert_eq!(state, state.clone());
}

#[test]
fn test_transaction_state_display() {
  assert_eq!(format!("{}", TransactionState::Active), "active");
  assert_eq!(format!("{}", TransactionState::Committed), "committed");
  assert_eq!(format!("{}", TransactionState::RolledBack), "rolled_back");
  assert_eq!(format!("{}", TransactionState::TimedOut), "timed_out");
}

#[test]
fn test_transaction_state_debug() {
  let state = TransactionState::Active;
  let _ = format!("{:?}", state);
}

// ============================================================================
// TransactionError Tests
// ============================================================================

#[test]
fn test_transaction_error_not_found() {
  let err = TransactionError::NotFound(TransactionId::new(1));
  assert_eq!(format!("{}", err), "Transaction not found: tx:1");
}

#[test]
fn test_transaction_error_not_active() {
  let err = TransactionError::NotActive(TransactionId::new(1), TransactionState::Committed);
  assert!(format!("{}", err).contains("is not active"));
}

#[test]
fn test_transaction_error_timeout() {
  let err = TransactionError::Timeout(TransactionId::new(1));
  assert_eq!(format!("{}", err), "Transaction tx:1 timed out");
}

#[test]
fn test_transaction_error_offset_error() {
  let offset_err = OffsetError::SourceNotFound("test".to_string());
  let err = TransactionError::OffsetError(offset_err);
  assert!(format!("{}", err).contains("Offset error"));
}

#[test]
fn test_transaction_error_from_offset_error() {
  let offset_err = OffsetError::SourceNotFound("test".to_string());
  let tx_err: TransactionError = offset_err.into();
  match tx_err {
    TransactionError::OffsetError(_) => {}
    _ => panic!("Expected OffsetError variant"),
  }
}

#[test]
fn test_transaction_error_lock_error() {
  let err = TransactionError::LockError("failed".to_string());
  assert_eq!(format!("{}", err), "Lock error: failed");
}

#[test]
fn test_transaction_error_invalid_operation() {
  let err = TransactionError::InvalidOperation("bad op".to_string());
  assert_eq!(format!("{}", err), "Invalid operation: bad op");
}

#[test]
fn test_transaction_error_savepoint_not_found() {
  let err = TransactionError::SavepointNotFound("sp1".to_string());
  assert_eq!(format!("{}", err), "Savepoint not found: sp1");
}

#[test]
fn test_transaction_error_nesting_limit_exceeded() {
  let err = TransactionError::NestingLimitExceeded(10);
  assert_eq!(format!("{}", err), "Nested transaction limit exceeded: 10");
}

#[test]
fn test_transaction_error_debug() {
  let err = TransactionError::NotFound(TransactionId::new(1));
  let _ = format!("{:?}", err);
}

#[test]
fn test_transaction_error_error_trait() {
  let err = TransactionError::NotFound(TransactionId::new(1));
  // TransactionError implements std::error::Error
  let _ = format!("{}", err);
}

// ============================================================================
// TransactionConfig Tests
// ============================================================================

#[test]
fn test_transaction_config_default() {
  let config = TransactionConfig::default();
  assert_eq!(config.timeout, Duration::from_secs(60));
  assert_eq!(config.max_nesting_level, 10);
  assert!(config.auto_rollback_on_timeout);
}

#[test]
fn test_transaction_config_new() {
  let config = TransactionConfig::new();
  assert_eq!(config.timeout, Duration::from_secs(60));
}

#[test]
fn test_transaction_config_with_timeout() {
  let timeout_duration = Duration::from_secs(30);
  let config = TransactionConfig::default().with_timeout(timeout_duration);
  assert_eq!(config.timeout, timeout_duration);
}

#[test]
fn test_transaction_config_with_max_nesting_level() {
  let config = TransactionConfig::default().with_max_nesting_level(20);
  assert_eq!(config.max_nesting_level, 20);
}

#[test]
fn test_transaction_config_with_auto_rollback_on_timeout() {
  let config = TransactionConfig::default().with_auto_rollback_on_timeout(false);
  assert!(!config.auto_rollback_on_timeout);
}

#[test]
fn test_transaction_config_builder_chain() {
  let config = TransactionConfig::default()
    .with_timeout(Duration::from_secs(30))
    .with_max_nesting_level(20)
    .with_auto_rollback_on_timeout(false);
  assert_eq!(config.timeout, Duration::from_secs(30));
  assert_eq!(config.max_nesting_level, 20);
  assert!(!config.auto_rollback_on_timeout);
}

#[test]
fn test_transaction_config_clone() {
  let config1 = TransactionConfig::default().with_timeout(Duration::from_secs(30));
  let config2 = config1.clone();
  assert_eq!(config1.timeout, config2.timeout);
}

#[test]
fn test_transaction_config_debug() {
  let config = TransactionConfig::default();
  let _ = format!("{:?}", config);
}

// ============================================================================
// Savepoint Tests
// ============================================================================

#[test]
fn test_savepoint_fields() {
  let savepoint = Savepoint {
    name: "sp1".to_string(),
    buffer_index: 5,
    created_at: std::time::Instant::now(),
  };
  assert_eq!(savepoint.name, "sp1");
  assert_eq!(savepoint.buffer_index, 5);
}

#[test]
fn test_savepoint_clone() {
  let savepoint1 = Savepoint {
    name: "sp1".to_string(),
    buffer_index: 5,
    created_at: std::time::Instant::now(),
  };
  let savepoint2 = savepoint1.clone();
  assert_eq!(savepoint1.name, savepoint2.name);
  assert_eq!(savepoint1.buffer_index, savepoint2.buffer_index);
}

#[test]
fn test_savepoint_debug() {
  let savepoint = Savepoint {
    name: "sp1".to_string(),
    buffer_index: 5,
    created_at: std::time::Instant::now(),
  };
  let _ = format!("{:?}", savepoint);
}

// ============================================================================
// BufferedOffset Tests
// ============================================================================

#[test]
fn test_buffered_offset_fields() {
  let buffered = BufferedOffset {
    source: "source1".to_string(),
    offset: Offset::Sequence(100),
  };
  assert_eq!(buffered.source, "source1");
  assert_eq!(buffered.offset, Offset::Sequence(100));
}

#[test]
fn test_buffered_offset_clone() {
  let buffered1 = BufferedOffset {
    source: "source1".to_string(),
    offset: Offset::Sequence(100),
  };
  let buffered2 = buffered1.clone();
  assert_eq!(buffered1.source, buffered2.source);
  assert_eq!(buffered1.offset, buffered2.offset);
}

#[test]
fn test_buffered_offset_debug() {
  let buffered = BufferedOffset {
    source: "source1".to_string(),
    offset: Offset::Sequence(100),
  };
  let _ = format!("{:?}", buffered);
}

// ============================================================================
// Transaction Tests
// ============================================================================

#[test]
fn test_transaction_new() {
  let id = TransactionId::new(1);
  let timeout = Duration::from_secs(60);
  let tx = Transaction::new(id, timeout);
  assert_eq!(tx.id, id);
  assert_eq!(tx.state, TransactionState::Active);
  assert!(tx.buffered_offsets.is_empty());
  assert!(tx.savepoints.is_empty());
  assert_eq!(tx.timeout, timeout);
  assert!(tx.metadata.is_empty());
}

#[test]
fn test_transaction_is_active() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  assert!(tx.is_active());
  tx.state = TransactionState::Committed;
  assert!(!tx.is_active());
}

#[test]
fn test_transaction_is_timed_out() {
  let timeout = Duration::from_millis(10);
  let tx = Transaction::new(TransactionId::new(1), timeout);
  assert!(!tx.is_timed_out());
  std::thread::sleep(Duration::from_millis(20));
  assert!(tx.is_timed_out());
}

#[test]
fn test_transaction_elapsed() {
  let tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  let elapsed = tx.elapsed();
  assert!(elapsed.as_millis() < 1000);
}

#[test]
fn test_transaction_remaining_time() {
  let timeout = Duration::from_secs(60);
  let tx = Transaction::new(TransactionId::new(1), timeout);
  let remaining = tx.remaining_time();
  assert!(remaining.is_some());
  assert!(remaining.unwrap().as_secs() < 60);
}

#[test]
fn test_transaction_buffer_offset() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  tx.buffer_offset("source1".to_string(), Offset::Sequence(100));
  assert_eq!(tx.buffered_offsets.len(), 1);
  assert_eq!(tx.buffered_offsets[0].source, "source1");
  assert_eq!(tx.buffered_offsets[0].offset, Offset::Sequence(100));
}

#[test]
fn test_transaction_create_savepoint() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  tx.buffer_offset("source1".to_string(), Offset::Sequence(100));
  let savepoint = tx.create_savepoint("sp1".to_string());
  assert_eq!(savepoint.name, "sp1");
  assert_eq!(savepoint.buffer_index, 1);
  assert_eq!(tx.savepoints.len(), 1);
}

#[test]
fn test_transaction_rollback_to_savepoint() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  tx.buffer_offset("source1".to_string(), Offset::Sequence(100));
  let _savepoint = tx.create_savepoint("sp1".to_string());
  tx.buffer_offset("source2".to_string(), Offset::Sequence(200));
  assert_eq!(tx.buffered_offsets.len(), 2);
  assert!(tx.rollback_to_savepoint("sp1").is_some());
  assert_eq!(tx.buffered_offsets.len(), 1);
  assert!(tx.savepoints.is_empty());
}

#[test]
fn test_transaction_rollback_to_savepoint_not_found() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  assert!(tx.rollback_to_savepoint("nonexistent").is_none());
}

#[test]
fn test_transaction_release_savepoint() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  let _savepoint = tx.create_savepoint("sp1".to_string());
  assert_eq!(tx.savepoints.len(), 1);
  let released = tx.release_savepoint("sp1");
  assert!(released.is_some());
  assert_eq!(released.unwrap().name, "sp1");
  assert!(tx.savepoints.is_empty());
}

#[test]
fn test_transaction_release_savepoint_not_found() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  assert!(tx.release_savepoint("nonexistent").is_none());
}

#[test]
fn test_transaction_set_get_metadata() {
  let mut tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  tx.set_metadata("key1", "value1");
  assert_eq!(tx.get_metadata("key1"), Some(&"value1".to_string()));
  assert_eq!(tx.get_metadata("nonexistent"), None);
}

#[test]
fn test_transaction_debug() {
  let tx = Transaction::new(TransactionId::new(1), Duration::from_secs(60));
  let _ = format!("{:?}", tx);
}

// ============================================================================
// TransactionManager Tests
// ============================================================================

#[tokio::test]
async fn test_transaction_manager_new() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default();
  let manager = TransactionManager::new(offset_store, config);
  assert_eq!(manager.config().timeout, Duration::from_secs(60));
}

#[tokio::test]
async fn test_transaction_manager_with_default_config() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::with_default_config(offset_store);
  assert_eq!(manager.config().timeout, Duration::from_secs(60));
}

#[tokio::test]
async fn test_transaction_manager_begin() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  assert_eq!(id.value(), 1);
}

#[tokio::test]
async fn test_transaction_manager_begin_with_timeout() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let custom_timeout = Duration::from_secs(30);
  let id = manager.begin_with_timeout(custom_timeout).await.unwrap();
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.elapsed.as_millis(), 0);
}

#[tokio::test]
async fn test_transaction_manager_buffer_offset() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1);
}

#[tokio::test]
async fn test_transaction_manager_commit() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager.commit(&id).await.unwrap();
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::Committed);
}

#[tokio::test]
async fn test_transaction_manager_commit_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let fake_id = TransactionId::new(999);
  let result = manager.commit(&fake_id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_transaction_manager_commit_not_active() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager.commit(&id).await.unwrap();
  let result = manager.commit(&id).await;
  assert!(matches!(result, Err(TransactionError::NotActive(_, _))));
}

#[tokio::test]
async fn test_transaction_manager_rollback() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager.rollback(&id).await.unwrap();
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::RolledBack);
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 0);
}

#[tokio::test]
async fn test_transaction_manager_rollback_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let fake_id = TransactionId::new(999);
  let result = manager.rollback(&fake_id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_transaction_manager_rollback_not_active() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager.rollback(&id).await.unwrap();
  let result = manager.rollback(&id).await;
  assert!(matches!(result, Err(TransactionError::NotActive(_, _))));
}

#[tokio::test]
async fn test_transaction_manager_create_savepoint() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let savepoint = manager.create_savepoint(&id, "sp1").await.unwrap();
  assert_eq!(savepoint.name, "sp1");
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.savepoint_count, 1);
}

#[tokio::test]
async fn test_transaction_manager_rollback_to_savepoint() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  let _savepoint = manager.create_savepoint(&id, "sp1").await.unwrap();
  manager
    .buffer_offset(&id, "source2", Offset::Sequence(200))
    .await
    .unwrap();
  manager.rollback_to_savepoint(&id, "sp1").await.unwrap();
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1);
  assert_eq!(info.savepoint_count, 0);
}

#[tokio::test]
async fn test_transaction_manager_rollback_to_savepoint_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let result = manager.rollback_to_savepoint(&id, "nonexistent").await;
  assert!(matches!(
    result,
    Err(TransactionError::SavepointNotFound(_))
  ));
}

#[tokio::test]
async fn test_transaction_manager_release_savepoint() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let _savepoint = manager.create_savepoint(&id, "sp1").await.unwrap();
  let released = manager.release_savepoint(&id, "sp1").await.unwrap();
  assert_eq!(released.name, "sp1");
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.savepoint_count, 0);
}

#[tokio::test]
async fn test_transaction_manager_release_savepoint_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let result = manager.release_savepoint(&id, "nonexistent").await;
  assert!(matches!(
    result,
    Err(TransactionError::SavepointNotFound(_))
  ));
}

#[tokio::test]
async fn test_transaction_manager_get_state() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::Active);
}

#[tokio::test]
async fn test_transaction_manager_get_state_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let fake_id = TransactionId::new(999);
  let result = manager.get_state(&fake_id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_transaction_manager_get_transaction() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.id, id);
  assert_eq!(info.state, TransactionState::Active);
  assert_eq!(info.buffered_offset_count, 0);
  assert_eq!(info.savepoint_count, 0);
  assert!(!info.is_timed_out);
}

#[tokio::test]
async fn test_transaction_manager_get_transaction_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let fake_id = TransactionId::new(999);
  let result = manager.get_transaction(&fake_id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_transaction_manager_list_active() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id1 = manager.begin().await.unwrap();
  let id2 = manager.begin().await.unwrap();
  let active = manager.list_active().await;
  assert!(active.contains(&id1));
  assert!(active.contains(&id2));
  manager.commit(&id1).await.unwrap();
  let active_after_commit = manager.list_active().await;
  assert!(!active_after_commit.contains(&id1));
  assert!(active_after_commit.contains(&id2));
}

#[tokio::test]
async fn test_transaction_manager_cleanup() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager.commit(&id).await.unwrap();
  sleep(Duration::from_millis(10)).await;
  let cleaned = manager.cleanup(Duration::from_millis(5)).await;
  assert_eq!(cleaned, 1);
  let result = manager.get_state(&id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_transaction_manager_check_timeouts() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default().with_timeout(Duration::from_millis(10));
  let manager = TransactionManager::new(offset_store, config);
  let id = manager.begin().await.unwrap();
  sleep(Duration::from_millis(20)).await;
  let timed_out = manager.check_timeouts().await;
  assert!(timed_out.contains(&id));
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::TimedOut);
}

#[tokio::test]
async fn test_transaction_manager_set_get_metadata() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager.set_metadata(&id, "key1", "value1").await.unwrap();
  let value = manager.get_metadata(&id, "key1").await.unwrap();
  assert_eq!(value, Some("value1".to_string()));
}

#[tokio::test]
async fn test_transaction_manager_get_metadata_not_found() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  let value = manager.get_metadata(&id, "nonexistent").await.unwrap();
  assert_eq!(value, None);
}

#[tokio::test]
async fn test_transaction_manager_buffer_offset_commits_on_commit() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&id, "source2", Offset::Sequence(200))
    .await
    .unwrap();
  manager.commit(&id).await.unwrap();
  // Verify transaction is committed (offsets are flushed)
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::Committed);
}

#[tokio::test]
async fn test_transaction_manager_nesting_limit() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default().with_max_nesting_level(2);
  let manager = TransactionManager::new(offset_store, config);
  let id = manager.begin().await.unwrap();
  let _sp1 = manager.create_savepoint(&id, "sp1").await.unwrap();
  let _sp2 = manager.create_savepoint(&id, "sp2").await.unwrap();
  let result = manager.create_savepoint(&id, "sp3").await;
  assert!(matches!(
    result,
    Err(TransactionError::NestingLimitExceeded(2))
  ));
}

#[tokio::test]
async fn test_transaction_manager_debug() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let _ = format!("{:?}", manager);
}

// ============================================================================
// TransactionInfo Tests
// ============================================================================

#[test]
fn test_transaction_info_fields() {
  let info = TransactionInfo {
    id: TransactionId::new(1),
    state: TransactionState::Active,
    buffered_offset_count: 5,
    savepoint_count: 2,
    elapsed: Duration::from_secs(1),
    remaining_time: Some(Duration::from_secs(59)),
    is_timed_out: false,
  };
  assert_eq!(info.id.value(), 1);
  assert_eq!(info.state, TransactionState::Active);
  assert_eq!(info.buffered_offset_count, 5);
  assert_eq!(info.savepoint_count, 2);
  assert_eq!(info.elapsed.as_secs(), 1);
  assert_eq!(info.remaining_time.unwrap().as_secs(), 59);
  assert!(!info.is_timed_out);
}

#[test]
fn test_transaction_info_clone() {
  let info1 = TransactionInfo {
    id: TransactionId::new(1),
    state: TransactionState::Active,
    buffered_offset_count: 5,
    savepoint_count: 2,
    elapsed: Duration::from_secs(1),
    remaining_time: Some(Duration::from_secs(59)),
    is_timed_out: false,
  };
  let info2 = info1.clone();
  assert_eq!(info1.id, info2.id);
  assert_eq!(info1.state, info2.state);
}

#[test]
fn test_transaction_info_debug() {
  let info = TransactionInfo {
    id: TransactionId::new(1),
    state: TransactionState::Active,
    buffered_offset_count: 5,
    savepoint_count: 2,
    elapsed: Duration::from_secs(1),
    remaining_time: Some(Duration::from_secs(59)),
    is_timed_out: false,
  };
  let _ = format!("{:?}", info);
}

// ============================================================================
// TransactionalContext Tests
// ============================================================================

#[tokio::test]
async fn test_transactional_context_new() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  assert_eq!(ctx.id().value(), 1);
  let state = manager.get_state(ctx.id()).await.unwrap();
  assert_eq!(state, TransactionState::Active);
}

#[tokio::test]
async fn test_transactional_context_with_timeout() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let custom_timeout = Duration::from_secs(30);
  let ctx = TransactionalContext::with_timeout(&manager, custom_timeout)
    .await
    .unwrap();
  assert_eq!(ctx.id().value(), 1);
}

#[tokio::test]
async fn test_transactional_context_buffer_offset() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  ctx
    .buffer_offset("source1", Offset::Sequence(100))
    .await
    .unwrap();
  let info = manager.get_transaction(ctx.id()).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1);
}

#[tokio::test]
async fn test_transactional_context_savepoint() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  let savepoint = ctx.savepoint("sp1").await.unwrap();
  assert_eq!(savepoint.name, "sp1");
}

#[tokio::test]
async fn test_transactional_context_rollback_to() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  ctx
    .buffer_offset("source1", Offset::Sequence(100))
    .await
    .unwrap();
  let _savepoint = ctx.savepoint("sp1").await.unwrap();
  ctx
    .buffer_offset("source2", Offset::Sequence(200))
    .await
    .unwrap();
  ctx.rollback_to("sp1").await.unwrap();
  let info = manager.get_transaction(ctx.id()).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1);
}

#[tokio::test]
async fn test_transactional_context_commit() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  let id = *ctx.id();
  ctx
    .buffer_offset("source1", Offset::Sequence(100))
    .await
    .unwrap();
  ctx.commit().await.unwrap();
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::Committed);
}

#[tokio::test]
async fn test_transactional_context_rollback() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  let id = *ctx.id();
  ctx
    .buffer_offset("source1", Offset::Sequence(100))
    .await
    .unwrap();
  ctx.rollback().await.unwrap();
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::RolledBack);
}

#[tokio::test]
async fn test_transactional_context_id() {
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(offset_store, TransactionConfig::default());
  let ctx = TransactionalContext::new(&manager).await.unwrap();
  let id = ctx.id();
  assert_eq!(id.value(), 1);
}

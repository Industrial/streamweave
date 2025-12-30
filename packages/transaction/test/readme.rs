//! Integration tests that verify all examples in README.md work correctly.
//!
//! These tests recreate the examples from the README.md file to ensure they
//! work as advertised. Each test corresponds to a code example in the README.

use std::time::Duration;
use streamweave_offset::{InMemoryOffsetStore, Offset};
use streamweave_transaction::{TransactionConfig, TransactionManager, TransactionalContext};

/// Test: Basic Transaction
///
/// This test recreates the "Basic Transaction" example from README.md lines 34-53.
#[tokio::test]
async fn test_basic_transaction() {
  // Example from README.md lines 34-53
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let config = TransactionConfig::default().with_timeout(Duration::from_secs(30));
  let manager = TransactionManager::new(offset_store, config);

  // Begin a transaction
  let tx_id = manager.begin().await.unwrap();

  // Buffer some offset commits
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "source2", Offset::Sequence(200))
    .await
    .unwrap();

  // Commit the transaction (flushes all buffered offsets)
  manager.commit(&tx_id).await.unwrap();
}

/// Test: Transactional Context
///
/// This test recreates the "Transactional Context" example from README.md lines 57-69.
#[tokio::test]
async fn test_transactional_context() {
  // Example from README.md lines 57-69
  let offset_store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::with_default_config(offset_store);

  let ctx = TransactionalContext::new(&manager).await.unwrap();

  // Buffer offsets
  ctx
    .buffer_offset("source1", Offset::Sequence(100))
    .await
    .unwrap();
  ctx
    .buffer_offset("source2", Offset::Sequence(200))
    .await
    .unwrap();

  // Commit (or rollback on error)
  ctx.commit().await.unwrap();
}

/// Test: Basic Transaction Flow
///
/// This test recreates the "Basic Transaction Flow" example from README.md lines 143-160.
#[tokio::test]
async fn test_basic_transaction_flow() {
  // Example from README.md lines 143-160
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  // Begin transaction
  let tx_id = manager.begin().await.unwrap();

  // Buffer multiple offset commits
  manager
    .buffer_offset(&tx_id, "topic-1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "topic-2", Offset::Sequence(200))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "topic-3", Offset::Sequence(300))
    .await
    .unwrap();

  // Commit (all offsets are flushed atomically)
  manager.commit(&tx_id).await.unwrap();

  // Verify offsets were committed
  assert_eq!(
    store_clone.get("topic-1").unwrap(),
    Some(Offset::Sequence(100))
  );
  assert_eq!(
    store_clone.get("topic-2").unwrap(),
    Some(Offset::Sequence(200))
  );
  assert_eq!(
    store_clone.get("topic-3").unwrap(),
    Some(Offset::Sequence(300))
  );
}

/// Test: Transaction Rollback
///
/// This test recreates the "Transaction Rollback" example from README.md lines 164-176.
#[tokio::test]
async fn test_transaction_rollback() {
  // Example from README.md lines 164-176
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  let tx_id = manager.begin().await.unwrap();

  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "source2", Offset::Sequence(200))
    .await
    .unwrap();

  // Simulate processing failure
  let processing_failed = true;

  // On error, rollback (all buffered offsets are discarded)
  if processing_failed {
    manager.rollback(&tx_id).await.unwrap();
  } else {
    manager.commit(&tx_id).await.unwrap();
  }

  // Verify offsets were not committed
  assert!(store_clone.get("source1").unwrap().is_none());
  assert!(store_clone.get("source2").unwrap().is_none());
}

/// Test: Savepoints
///
/// This test recreates the "Savepoints" example from README.md lines 180-204.
#[tokio::test]
async fn test_savepoints() {
  // Example from README.md lines 180-204
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  let tx_id = manager.begin().await.unwrap();

  // Process first batch
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "source2", Offset::Sequence(200))
    .await
    .unwrap();

  // Create savepoint
  manager.create_savepoint(&tx_id, "batch1").await.unwrap();

  // Process second batch
  manager
    .buffer_offset(&tx_id, "source3", Offset::Sequence(300))
    .await
    .unwrap();
  manager
    .buffer_offset(&tx_id, "source4", Offset::Sequence(400))
    .await
    .unwrap();

  // Simulate second batch failure
  let batch2_failed = true;

  // If second batch fails, rollback to savepoint
  if batch2_failed {
    manager
      .rollback_to_savepoint(&tx_id, "batch1")
      .await
      .unwrap();
    // Only batch1 offsets remain buffered
  }

  // Commit (only successful batches)
  manager.commit(&tx_id).await.unwrap();

  // Verify only first batch offsets were committed
  assert_eq!(
    store_clone.get("source1").unwrap(),
    Some(Offset::Sequence(100))
  );
  assert_eq!(
    store_clone.get("source2").unwrap(),
    Some(Offset::Sequence(200))
  );
  assert!(store_clone.get("source3").unwrap().is_none());
  assert!(store_clone.get("source4").unwrap().is_none());
}

/// Test: Transaction Timeouts
///
/// This test recreates the "Transaction Timeouts" example from README.md lines 208-227.
#[tokio::test]
async fn test_transaction_timeouts() {
  // Example from README.md lines 208-227
  let config = TransactionConfig::default()
    .with_timeout(Duration::from_millis(50))
    .with_auto_rollback_on_timeout(true);

  let store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(store, config);

  let tx_id = manager.begin().await.unwrap();

  // Transaction will auto-rollback if it exceeds timeout
  // Wait for timeout
  tokio::time::sleep(Duration::from_millis(60)).await;

  // Check timeout status
  let state = manager.get_state(&tx_id).await.unwrap();
  assert_eq!(state, streamweave_transaction::TransactionState::TimedOut);
}

/// Test: Transactional Context (Full Example)
///
/// This test recreates the "Transactional Context" example from README.md lines 231-259.
#[tokio::test]
async fn test_transactional_context_full() {
  // Example from README.md lines 231-259
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  async fn process_batch(
    manager: &TransactionManager,
  ) -> Result<(), streamweave_transaction::TransactionError> {
    let ctx = TransactionalContext::new(manager).await?;

    // Buffer offsets
    ctx.buffer_offset("source1", Offset::Sequence(100)).await?;
    ctx.buffer_offset("source2", Offset::Sequence(200)).await?;

    // Create savepoint
    ctx.savepoint("checkpoint").await?;

    // More operations
    ctx.buffer_offset("source3", Offset::Sequence(300)).await?;

    // Simulate error
    let error_occurred = false; // Set to true to test rollback

    // On error, rollback to savepoint
    if error_occurred {
      ctx.rollback_to("checkpoint").await?;
    }

    // Commit (or rollback)
    ctx.commit().await?;

    Ok(())
  }

  process_batch(&manager).await.unwrap();

  // Verify offsets were committed
  assert_eq!(
    store_clone.get("source1").unwrap(),
    Some(Offset::Sequence(100))
  );
  assert_eq!(
    store_clone.get("source2").unwrap(),
    Some(Offset::Sequence(200))
  );
  assert_eq!(
    store_clone.get("source3").unwrap(),
    Some(Offset::Sequence(300))
  );
}

/// Test: Custom Timeout per Transaction
///
/// This test recreates the "Custom Timeout per Transaction" example from README.md lines 263-271.
#[tokio::test]
async fn test_custom_timeout_per_transaction() {
  // Example from README.md lines 263-271
  let store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::with_default_config(store);

  let tx_id = manager
    .begin_with_timeout(Duration::from_secs(60))
    .await
    .unwrap();

  // This transaction has 60 second timeout
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager.commit(&tx_id).await.unwrap();
}

/// Test: Transaction Metadata
///
/// This test recreates the "Transaction Metadata" example from README.md lines 275-285.
#[tokio::test]
async fn test_transaction_metadata() {
  // Example from README.md lines 275-285
  let store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::with_default_config(store);

  let tx_id = manager.begin().await.unwrap();

  manager
    .set_metadata(&tx_id, "user_id", "123")
    .await
    .unwrap();
  manager
    .set_metadata(&tx_id, "operation", "batch_process")
    .await
    .unwrap();

  // Retrieve metadata
  let user_id = manager.get_metadata(&tx_id, "user_id").await.unwrap();
  assert_eq!(user_id, Some("123".to_string()));

  let operation = manager.get_metadata(&tx_id, "operation").await.unwrap();
  assert_eq!(operation, Some("batch_process".to_string()));
}

/// Test: Transaction Monitoring
///
/// This test recreates the "Transaction Monitoring" example from README.md lines 291-302.
#[tokio::test]
async fn test_transaction_monitoring() {
  // Example from README.md lines 291-302
  let store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::with_default_config(store);

  // List all active transactions
  let active = manager.list_active().await;
  assert_eq!(active.len(), 0); // Initially empty

  // Begin a transaction
  let tx_id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&tx_id, "source1", Offset::Sequence(100))
    .await
    .unwrap();

  // Get transaction info
  let info = manager.get_transaction(&tx_id).await.unwrap();
  assert_eq!(info.id, tx_id);
  assert_eq!(info.buffered_offset_count, 1);
  assert!(info.elapsed.as_secs() >= 0);
  assert!(info.remaining_time.is_some());
}

/// Test: Timeout Checking
///
/// This test recreates the "Timeout Checking" example from README.md lines 308-319.
#[tokio::test]
async fn test_timeout_checking() {
  // Example from README.md lines 308-319
  let config = TransactionConfig::default()
    .with_timeout(Duration::from_millis(50))
    .with_auto_rollback_on_timeout(true);
  let store = Box::new(InMemoryOffsetStore::new());
  let manager = TransactionManager::new(store, config);

  let tx_id = manager.begin().await.unwrap();

  // Wait for timeout
  tokio::time::sleep(Duration::from_millis(60)).await;

  // Check for timed-out transactions
  let timed_out = manager.check_timeouts().await;
  assert_eq!(timed_out.len(), 1);
  assert!(timed_out.contains(&tx_id));

  // Cleanup old completed transactions
  let cleaned = manager.cleanup(Duration::from_secs(3600)).await;
  assert_eq!(cleaned, 0); // Transaction is still active (timed out but not cleaned yet)
}

/// Test: Nested Savepoints
///
/// This test recreates the "Nested Savepoints" example from README.md lines 325-343.
#[tokio::test]
async fn test_nested_savepoints() {
  // Example from README.md lines 325-343
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  let tx_id = manager.begin().await.unwrap();

  // First checkpoint
  manager
    .buffer_offset(&tx_id, "s1", Offset::Sequence(1))
    .await
    .unwrap();
  manager
    .create_savepoint(&tx_id, "checkpoint1")
    .await
    .unwrap();

  // Second checkpoint
  manager
    .buffer_offset(&tx_id, "s2", Offset::Sequence(2))
    .await
    .unwrap();
  manager
    .create_savepoint(&tx_id, "checkpoint2")
    .await
    .unwrap();

  // Third checkpoint
  manager
    .buffer_offset(&tx_id, "s3", Offset::Sequence(3))
    .await
    .unwrap();
  manager
    .create_savepoint(&tx_id, "checkpoint3")
    .await
    .unwrap();

  // Rollback to any checkpoint
  manager
    .rollback_to_savepoint(&tx_id, "checkpoint1")
    .await
    .unwrap();
  // Only s1 remains buffered

  // Commit
  manager.commit(&tx_id).await.unwrap();

  // Verify only s1 was committed
  assert_eq!(store_clone.get("s1").unwrap(), Some(Offset::Sequence(1)));
  assert!(store_clone.get("s2").unwrap().is_none());
  assert!(store_clone.get("s3").unwrap().is_none());
}

use futures::future;
use std::time::Duration;
use streamweave_offset::{InMemoryOffsetStore, Offset};
use streamweave_transaction::*;

fn create_manager() -> TransactionManager {
  let store = Box::new(InMemoryOffsetStore::new());
  TransactionManager::with_default_config(store)
}

fn create_manager_with_config(config: TransactionConfig) -> TransactionManager {
  let store = Box::new(InMemoryOffsetStore::new());
  TransactionManager::new(store, config)
}

#[tokio::test]
async fn test_transaction_id_display() {
  let id = TransactionId::new(42);
  assert_eq!(format!("{}", id), "tx:42");
  assert_eq!(id.value(), 42);
}

#[tokio::test]
async fn test_transaction_state_display() {
  assert_eq!(format!("{}", TransactionState::Active), "active");
  assert_eq!(format!("{}", TransactionState::Committed), "committed");
  assert_eq!(format!("{}", TransactionState::RolledBack), "rolled_back");
  assert_eq!(format!("{}", TransactionState::TimedOut), "timed_out");
}

#[tokio::test]
async fn test_begin_commit() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  assert_eq!(
    manager.get_state(&id).await.unwrap(),
    TransactionState::Active
  );

  manager.commit(&id).await.unwrap();
  assert_eq!(
    manager.get_state(&id).await.unwrap(),
    TransactionState::Committed
  );
}

#[tokio::test]
async fn test_begin_rollback() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  manager.rollback(&id).await.unwrap();

  assert_eq!(
    manager.get_state(&id).await.unwrap(),
    TransactionState::RolledBack
  );
}

#[tokio::test]
async fn test_buffer_and_commit_offsets() {
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  let id = manager.begin().await.unwrap();

  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager
    .buffer_offset(&id, "source2", Offset::Sequence(200))
    .await
    .unwrap();

  // Before commit, offsets should not be in store
  assert!(store_clone.get("source1").unwrap().is_none());
  assert!(store_clone.get("source2").unwrap().is_none());

  manager.commit(&id).await.unwrap();

  // After commit, offsets should be in store
  assert_eq!(
    store_clone.get("source1").unwrap(),
    Some(Offset::Sequence(100))
  );
  assert_eq!(
    store_clone.get("source2").unwrap(),
    Some(Offset::Sequence(200))
  );
}

#[tokio::test]
async fn test_rollback_discards_offsets() {
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  let id = manager.begin().await.unwrap();

  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();
  manager.rollback(&id).await.unwrap();

  // Offsets should not be committed
  assert!(store_clone.get("source1").unwrap().is_none());
}

#[tokio::test]
async fn test_savepoint_and_rollback() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();

  manager
    .buffer_offset(&id, "s1", Offset::Sequence(1))
    .await
    .unwrap();
  manager.create_savepoint(&id, "sp1").await.unwrap();

  manager
    .buffer_offset(&id, "s2", Offset::Sequence(2))
    .await
    .unwrap();
  manager
    .buffer_offset(&id, "s3", Offset::Sequence(3))
    .await
    .unwrap();

  // Rollback to savepoint should discard s2 and s3
  manager.rollback_to_savepoint(&id, "sp1").await.unwrap();

  // Get transaction info to check buffered count
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1); // Only s1 remains
}

#[tokio::test]
async fn test_savepoint_nesting_limit() {
  let config = TransactionConfig::default().with_max_nesting_level(2);
  let manager = create_manager_with_config(config);

  let id = manager.begin().await.unwrap();

  manager.create_savepoint(&id, "sp1").await.unwrap();
  manager.create_savepoint(&id, "sp2").await.unwrap();

  // Third savepoint should fail
  let result = manager.create_savepoint(&id, "sp3").await;
  assert!(matches!(
    result,
    Err(TransactionError::NestingLimitExceeded(2))
  ));
}

#[tokio::test]
async fn test_transaction_timeout() {
  let config = TransactionConfig::default().with_timeout(Duration::from_millis(10));
  let manager = create_manager_with_config(config);

  let id = manager.begin().await.unwrap();

  // Wait for timeout
  tokio::time::sleep(Duration::from_millis(20)).await;

  // Check state should detect timeout
  let state = manager.get_state(&id).await.unwrap();
  assert_eq!(state, TransactionState::TimedOut);
}

#[tokio::test]
async fn test_cannot_commit_non_active() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  manager.rollback(&id).await.unwrap();

  // Should fail to commit rolled back transaction
  let result = manager.commit(&id).await;
  assert!(matches!(
    result,
    Err(TransactionError::NotActive(_, TransactionState::RolledBack))
  ));
}

#[tokio::test]
async fn test_begin_with_timeout() {
  let manager = create_manager();
  let id = manager
    .begin_with_timeout(Duration::from_millis(50))
    .await
    .unwrap();

  let info = manager.get_transaction(&id).await.unwrap();
  assert!(info.remaining_time.is_some());
  assert!(info.remaining_time.unwrap() <= Duration::from_millis(50));
}

#[tokio::test]
async fn test_transaction_config_with_auto_rollback() {
  let config = TransactionConfig::default().with_auto_rollback_on_timeout(true);
  assert!(config.auto_rollback_on_timeout);

  let config = TransactionConfig::default().with_auto_rollback_on_timeout(false);
  assert!(!config.auto_rollback_on_timeout);
}

#[tokio::test]
async fn test_transaction_elapsed_and_remaining_time() {
  let manager = create_manager();
  let id = manager.begin().await.unwrap();

  // Wait a bit
  tokio::time::sleep(Duration::from_millis(10)).await;

  let info = manager.get_transaction(&id).await.unwrap();
  assert!(info.elapsed >= Duration::from_millis(10));
  assert!(info.remaining_time.is_some());
}

#[tokio::test]
async fn test_transactional_context_with_timeout() {
  let manager = create_manager();

  let ctx = TransactionalContext::with_timeout(&manager, Duration::from_secs(30))
    .await
    .unwrap();
  let id = *ctx.id();

  assert_eq!(
    manager.get_state(&id).await.unwrap(),
    TransactionState::Active
  );
}

#[tokio::test]
async fn test_transactional_context_savepoint() {
  let manager = create_manager();

  let ctx = TransactionalContext::new(&manager).await.unwrap();
  ctx.buffer_offset("s1", Offset::Sequence(1)).await.unwrap();
  ctx.savepoint("sp1").await.unwrap();
  ctx.buffer_offset("s2", Offset::Sequence(2)).await.unwrap();
  ctx.rollback_to("sp1").await.unwrap();

  let id = *ctx.id();
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 1);
}

#[tokio::test]
async fn test_cannot_rollback_non_active() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  manager.commit(&id).await.unwrap();

  // Should fail to rollback committed transaction
  let result = manager.rollback(&id).await;
  assert!(matches!(
    result,
    Err(TransactionError::NotActive(_, TransactionState::Committed))
  ));
}

#[tokio::test]
async fn test_transaction_not_found() {
  let manager = create_manager();

  let fake_id = TransactionId::new(999);
  let result = manager.commit(&fake_id).await;
  assert!(matches!(result, Err(TransactionError::NotFound(_))));
}

#[tokio::test]
async fn test_list_active_transactions() {
  let manager = create_manager();

  let id1 = manager.begin().await.unwrap();
  let id2 = manager.begin().await.unwrap();
  let id3 = manager.begin().await.unwrap();

  manager.commit(&id2).await.unwrap();

  let active = manager.list_active().await;
  assert_eq!(active.len(), 2);
  assert!(active.contains(&id1));
  assert!(active.contains(&id3));
  assert!(!active.contains(&id2));
}

#[tokio::test]
async fn test_cleanup_completed_transactions() {
  let manager = create_manager();

  let id1 = manager.begin().await.unwrap();
  let id2 = manager.begin().await.unwrap();

  manager.commit(&id1).await.unwrap();
  manager.rollback(&id2).await.unwrap();

  // Cleanup with 0 duration should remove completed transactions
  let cleaned = manager.cleanup(Duration::ZERO).await;
  assert_eq!(cleaned, 2);

  // Both should now be not found
  assert!(matches!(
    manager.get_state(&id1).await,
    Err(TransactionError::NotFound(_))
  ));
  assert!(matches!(
    manager.get_state(&id2).await,
    Err(TransactionError::NotFound(_))
  ));
}

#[tokio::test]
async fn test_check_timeouts() {
  let config = TransactionConfig::default()
    .with_timeout(Duration::from_millis(10))
    .with_auto_rollback_on_timeout(true);
  let manager = create_manager_with_config(config);

  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "source1", Offset::Sequence(100))
    .await
    .unwrap();

  // Wait for timeout
  tokio::time::sleep(Duration::from_millis(20)).await;

  let timed_out = manager.check_timeouts().await;
  assert_eq!(timed_out.len(), 1);
  assert!(timed_out.contains(&id));

  // Transaction should be marked as timed out
  assert_eq!(
    manager.get_state(&id).await.unwrap(),
    TransactionState::TimedOut
  );

  // Buffered offsets should be cleared (auto-rollback)
  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.buffered_offset_count, 0);
}

#[tokio::test]
async fn test_transaction_metadata() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();

  manager.set_metadata(&id, "user_id", "123").await.unwrap();
  manager
    .set_metadata(&id, "operation", "batch_process")
    .await
    .unwrap();

  assert_eq!(
    manager.get_metadata(&id, "user_id").await.unwrap(),
    Some("123".to_string())
  );
  assert_eq!(
    manager.get_metadata(&id, "operation").await.unwrap(),
    Some("batch_process".to_string())
  );
  assert_eq!(
    manager.get_metadata(&id, "nonexistent").await.unwrap(),
    None
  );
}

#[tokio::test]
async fn test_transaction_info() {
  let config = TransactionConfig::default().with_timeout(Duration::from_secs(30));
  let manager = create_manager_with_config(config);

  let id = manager.begin().await.unwrap();
  manager
    .buffer_offset(&id, "s1", Offset::Sequence(1))
    .await
    .unwrap();
  manager
    .buffer_offset(&id, "s2", Offset::Sequence(2))
    .await
    .unwrap();
  manager.create_savepoint(&id, "sp1").await.unwrap();

  let info = manager.get_transaction(&id).await.unwrap();

  assert_eq!(info.id, id);
  assert_eq!(info.state, TransactionState::Active);
  assert_eq!(info.buffered_offset_count, 2);
  assert_eq!(info.savepoint_count, 1);
  assert!(!info.is_timed_out);
  assert!(info.remaining_time.is_some());
}

#[tokio::test]
async fn test_release_savepoint() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  manager.create_savepoint(&id, "sp1").await.unwrap();
  manager.create_savepoint(&id, "sp2").await.unwrap();

  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.savepoint_count, 2);

  manager.release_savepoint(&id, "sp1").await.unwrap();

  let info = manager.get_transaction(&id).await.unwrap();
  assert_eq!(info.savepoint_count, 1);
}

#[tokio::test]
async fn test_release_nonexistent_savepoint() {
  let manager = create_manager();

  let id = manager.begin().await.unwrap();
  let result = manager.release_savepoint(&id, "nonexistent").await;

  assert!(matches!(
    result,
    Err(TransactionError::SavepointNotFound(_))
  ));
}

#[tokio::test]
async fn test_config_builder() {
  let config = TransactionConfig::new()
    .with_timeout(Duration::from_secs(120))
    .with_max_nesting_level(5)
    .with_auto_rollback_on_timeout(false);

  assert_eq!(config.timeout, Duration::from_secs(120));
  assert_eq!(config.max_nesting_level, 5);
  assert!(!config.auto_rollback_on_timeout);
}

#[tokio::test]
async fn test_transactional_context() {
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  {
    let ctx = TransactionalContext::new(&manager).await.unwrap();
    ctx
      .buffer_offset("source1", Offset::Sequence(100))
      .await
      .unwrap();
    ctx.commit().await.unwrap();
  }

  assert_eq!(
    store_clone.get("source1").unwrap(),
    Some(Offset::Sequence(100))
  );
}

#[tokio::test]
async fn test_transactional_context_with_savepoint() {
  let store = Box::new(InMemoryOffsetStore::new());
  let store_clone = store.clone();
  let manager = TransactionManager::with_default_config(store);

  {
    let ctx = TransactionalContext::new(&manager).await.unwrap();
    ctx
      .buffer_offset("source1", Offset::Sequence(100))
      .await
      .unwrap();
    ctx.savepoint("sp1").await.unwrap();
    ctx
      .buffer_offset("source2", Offset::Sequence(200))
      .await
      .unwrap();
    ctx.rollback_to("sp1").await.unwrap();
    ctx.commit().await.unwrap();
  }

  // Only source1 should be committed (source2 was after savepoint)
  assert_eq!(
    store_clone.get("source1").unwrap(),
    Some(Offset::Sequence(100))
  );
  assert!(store_clone.get("source2").unwrap().is_none());
}

#[tokio::test]
async fn test_transaction_error_display() {
  let id = TransactionId::new(1);

  let not_found = TransactionError::NotFound(id);
  assert!(not_found.to_string().contains("not found"));

  let not_active = TransactionError::NotActive(id, TransactionState::Committed);
  assert!(not_active.to_string().contains("not active"));

  let timeout = TransactionError::Timeout(id);
  assert!(timeout.to_string().contains("timed out"));

  let invalid = TransactionError::InvalidOperation("test".to_string());
  assert!(invalid.to_string().contains("Invalid operation"));

  let savepoint = TransactionError::SavepointNotFound("sp1".to_string());
  assert!(savepoint.to_string().contains("Savepoint not found"));

  let nesting = TransactionError::NestingLimitExceeded(10);
  assert!(nesting.to_string().contains("limit exceeded"));
}

#[tokio::test]
async fn test_multiple_transactions_concurrent() {
  let manager = create_manager();

  // Begin multiple transactions
  let ids: Vec<_> = future::join_all((0..10).map(|_| manager.begin()))
    .await
    .into_iter()
    .map(|r| r.unwrap())
    .collect();

  // All should be active
  for id in &ids {
    assert_eq!(
      manager.get_state(id).await.unwrap(),
      TransactionState::Active
    );
  }

  // Commit half, rollback half
  for (i, id) in ids.iter().enumerate() {
    if i % 2 == 0 {
      manager.commit(id).await.unwrap();
    } else {
      manager.rollback(id).await.unwrap();
    }
  }

  // Verify states
  for (i, id) in ids.iter().enumerate() {
    let expected = if i % 2 == 0 {
      TransactionState::Committed
    } else {
      TransactionState::RolledBack
    };
    assert_eq!(manager.get_state(id).await.unwrap(), expected);
  }
}

#[tokio::test]
async fn test_transaction_remaining_time() {
  let config = TransactionConfig::default().with_timeout(Duration::from_secs(10));
  let manager = create_manager_with_config(config);

  let id = manager.begin().await.unwrap();

  let info = manager.get_transaction(&id).await.unwrap();
  assert!(info.remaining_time.is_some());

  let remaining = info.remaining_time.unwrap();
  assert!(remaining <= Duration::from_secs(10));
  assert!(remaining > Duration::from_secs(9)); // Should be close to 10s
}

//! # Exactly-once state
//!
//! Demonstrates [`KeyedStateBackend`], [`StatefulNodeDriver`], and idempotent
//! semantics: keyed state with logical-time versioning, snapshot/restore, and
//! an idempotent sink-style write pattern.
//!
//! See [docs/exactly-once-state.md](docs/exactly-once-state.md) and
//! [docs/EXAMPLES-AND-HOW-TO.md](docs/EXAMPLES-AND-HOW-TO.md).

use streamweave::state::{
  ExactlyOnceStateBackend, HashMapStateBackend, KeyedStateBackend, StatefulNodeDriver,
};
use streamweave::time::LogicalTime;

fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
  println!(
    "Exactly-once state example (KeyedStateBackend, StatefulNodeDriver, snapshot/restore)\n"
  );

  // KeyedStateBackend is HashMapStateBackend<K, V, LogicalTime>
  let backend: KeyedStateBackend<String, u64> = HashMapStateBackend::new();

  // --- StatefulNodeDriver: apply_update with LogicalTime (idempotent) ---
  println!("1. Applying updates with logical times (StatefulNodeDriver)");
  backend.apply_update("user:alice".to_string(), 100u64, LogicalTime::new(1))?;
  backend.apply_update("user:bob".to_string(), 200u64, LogicalTime::new(1))?;
  backend.apply_update("user:alice".to_string(), 150u64, LogicalTime::new(2))?;

  assert_eq!(
    backend.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2)))
  );
  assert_eq!(
    backend.get_value(&"user:bob".to_string())?,
    Some((200u64, LogicalTime::new(1)))
  );
  println!("   user:alice -> 150 @ t=2, user:bob -> 200 @ t=1");

  // --- Idempotency: same version applied again does not change state ---
  println!("\n2. Idempotent put (same version again is a no-op)");
  backend.apply_update("user:alice".to_string(), 999u64, LogicalTime::new(2))?;
  assert_eq!(
    backend.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2))),
    "re-applying at t=2 must not overwrite"
  );
  println!("   Re-applied (alice, 999, t=2) -> state unchanged (150 @ t=2)");

  // --- Snapshot and restore ---
  println!("\n3. Snapshot and restore");
  let bytes = backend.snapshot_bytes()?;
  println!("   Snapshot size: {} bytes", bytes.len());

  let mut restored: KeyedStateBackend<String, u64> = HashMapStateBackend::new();
  // restore is on ExactlyOnceStateBackend and takes &mut self; we use the inner type
  restored.restore(&bytes)?;
  assert_eq!(
    restored.get_value(&"user:alice".to_string())?,
    Some((150u64, LogicalTime::new(2)))
  );
  assert_eq!(
    restored.get_value(&"user:bob".to_string())?,
    Some((200u64, LogicalTime::new(1)))
  );
  println!("   Restored backend has same state as original");

  // --- Idempotent sink pattern: stable (key, version) for writes ---
  println!("\n4. Idempotent sink pattern (stable key + version per record)");
  let sink_state: KeyedStateBackend<String, u64> = HashMapStateBackend::new();
  for (key, value, time) in [
    ("rec:1".to_string(), 10u64, LogicalTime::new(1)),
    ("rec:2".to_string(), 20u64, LogicalTime::new(2)),
    ("rec:1".to_string(), 10u64, LogicalTime::new(1)), // duplicate on replay
  ] {
    sink_state.apply_update(key.clone(), value, time)?;
  }
  // After "replay", rec:1 still 10 @ t=1 (duplicate write was idempotent)
  assert_eq!(
    sink_state.get_value(&"rec:1".to_string())?,
    Some((10u64, LogicalTime::new(1)))
  );
  println!("   Duplicate (rec:1, 10, t=1) did not double-apply");

  println!("\nDone.");
  Ok(())
}

# Exactly-Once State

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §3.

**Dependencies:** None (optional: Logical timestamps for time-based dedup; checkpointing for recovery).

---

## 1. Objective and rationale

**Objective:** Each update to internal state is applied exactly once, and on recovery the state is consistent (no double-apply, no lost updates). Usually achieved with idempotent updates, deterministic replay, and checkpointing.

**Why it matters for StreamWeave:**

- Users expect “counts and aggregates are correct” after failures and restarts.
- At-least-once or at-most-once is insufficient for financial or analytical correctness.
- Exactly-once is a prerequisite for trusted stateful streaming and for distributed checkpointing later.

---

## 2. Current state in StreamWeave

- **PRD:** Mentions exactly-once as a goal (transactions, idempotency, dedup, checkpointing, offset tracking); implementation is not present.
- **Stateful nodes:** Nodes can hold internal state (e.g. in-memory `HashMap`), but there is no standard notion of “key,” “version,” or “sequence” for idempotency.
- **Replay:** No built-in replay or “apply (key, update, time) once” semantics.
- **Sinks:** No standard pattern for idempotent writes (e.g. “write with key/version; overwrite same key is idempotent”).
- **Checkpointing:** No checkpointing yet; see [distributed-checkpointing.md](distributed-checkpointing.md) for local then distributed checkpointing.

---

## 3. Core concepts

### 3.1 State identity

Stateful nodes should have a clear notion of:

- **Key:** The partition of state (e.g. user_id, session_id). Updates are key-scoped.
- **Version / sequence / logical time:** A monotonic identifier so that “apply (key, value, version)” is idempotent: applying the same (key, value, version) again has no effect, and applying an older version after a newer one can be ignored or defined (e.g. “last-writer-wins” or “merge”).

### 3.2 Idempotent state updates

Design state APIs so that applying the same `(key, update, logical_time)` multiple times is idempotent:

- **Put with version:** `put(key, value, version)`. If `version <= current_version(key)`, skip or overwrite only if version is strictly greater, depending on policy.
- **Append / log:** If state is a log, use a unique id (e.g. event_id) and deduplicate on read or on write (e.g. “insert if not exists by id”).

### 3.3 Checkpoint and replay

- **Checkpoint:** Periodically persist state (and optionally input positions) to durable storage.
- **Restore:** On restart, load state from the last checkpoint.
- **Replay:** Replay input from the last checkpoint; exactly-once state semantics ensure that replaying the same updates does not double-apply (because of version/time dedup).

### 3.4 Sinks

Exactly-once sinks require one of:

- **Idempotent writes:** e.g. “write with key/version; overwrite same key is idempotent.”
- **Transactional writes:** e.g. two-phase commit or “write to temp then rename to final” with a single commit step.
- **Deduplication at consumer:** Consumer tracks “last processed id” and skips duplicates (requires stable ids on messages).

---

## 4. Detailed design

### 4.1 Stateful node contract (proposed)

Introduce a **contract** (and optionally a trait or helpers) for “exactly-once stateful” nodes:

1. **Keyed state:** State is addressed by a **key** (e.g. `Vec<u8>` or a type that implements `Hash + Eq`). All updates are in terms of (key, value, optional version/time).
2. **Version or logical time:** Every update carries a **version** or **logical time**. The node guarantees that applying the same (key, value, version) again does not change the result (idempotent).
3. **Ordering:** Updates for the same key are processed in version order (or the node documents its ordering policy, e.g. “last-writer-wins”).
4. **Persistence:** For recovery, the node must be able to **serialize** its state (e.g. for checkpointing) and **deserialize** to restore.

**Rust API sketch:**

```rust
/// Trait for state backends that support exactly-once updates.
pub trait ExactlyOnceStateBackend {
    type Key: Hash + Eq + Clone + Send;
    type Value: Serialize + for<'de> Deserialize<'de> + Send;
    type Version: Ord + Clone + Send; // e.g. LogicalTime or u64

    /// Put value for key at version. Idempotent: if version <= current version for key, no-op or defined policy.
    fn put(&self, key: Self::Key, value: Self::Value, version: Self::Version) -> Result<(), Error>;

    /// Get current value and version for key.
    fn get(&self, key: &Self::Key) -> Result<Option<(Self::Value, Self::Version)>, Error>;

    /// Snapshot state for checkpoint (e.g. return serialized state).
    fn snapshot(&self) -> Result<Vec<u8>, Error>;

    /// Restore state from checkpoint.
    fn restore(&mut self, data: &[u8]) -> Result<(), Error>;
}
```

Nodes that implement this (or use a state store that does) can advertise “exactly-once state” and participate in checkpoint/restore.

### 4.2 Integration with logical time

- Use **LogicalTime** (or a pair epoch/round) as the **version** for state updates. When the execution layer assigns a logical time to each item, stateful nodes can use that time as the version for `put(key, value, time)`.
- On replay after checkpoint, the same (key, value, time) may be delivered again; the backend’s idempotency ensures no double-apply.

### 4.3 In-process first

- Implement for **in-process stateful nodes** first: e.g. a `HashMapStateBackend` or `RocksDbStateBackend` that supports `put(key, value, version)` and `snapshot`/`restore`.
- No distribution required initially; the same process checkpoints its state to local disk (or a shared store).

### 4.4 Sinks: idempotent write pattern

- **Document** a pattern for idempotent sinks: e.g. “each output record has a stable `(key, version)`; the sink writes with that key and version; overwrite is idempotent.”
- Optionally provide a **wrapper node** or helper that adds a deterministic “output record id” (e.g. hash of (input_key, logical_time)) so that downstream sinks can deduplicate.

### 4.5 Edge cases

- **Out-of-order delivery:** If updates for the same key arrive out of order (e.g. version 3 before version 2), the backend must either buffer and reorder, or apply “last-writer-wins” and accept that older versions overwrite newer until the correct order is established. For exactly-once, “apply in version order” is preferred; buffer by key until a version is “sealed” (e.g. no more updates with lower version).
- **Recovery:** After restore from checkpoint, replay starts from the checkpoint’s position. Any duplicate messages in the replay are idempotent; any missed messages must be handled by “at-least-once” delivery of the source (e.g. Kafka) so that replay includes them.

---

## 5. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Define and document the exactly-once state contract and key/version semantics. **Done:** `state::ExactlyOnceStateBackend` trait, README § Exactly-once state, [architecture.md](architecture.md#exactly-once-state). |
| **2** | Implement an in-process state backend with `put(key, value, version)`, `get`, `snapshot`, `restore` (e.g. in-memory + optional file snapshot). **Done:** `state::HashMapStateBackend<K, V, Ver>` in `src/state.rs`. |
| **3** | Add a “stateful node” helper or trait that uses this backend and is driven by logical time from the execution layer. |
| **4** | Document idempotent sink pattern and, if needed, add a small helper for stable output ids. |
| **5** | Integrate with local checkpointing (see [distributed-checkpointing.md](distributed-checkpointing.md)) so that state is included in checkpoints. |

---

## 6. References

- Gap analysis §3 (Exactly-once state).
- [distributed-checkpointing.md](distributed-checkpointing.md) for checkpoint/restore.
- [logical-timestamps-timely-and-streamweave.md](logical-timestamps-timely-and-streamweave.md) for LogicalTime as version.

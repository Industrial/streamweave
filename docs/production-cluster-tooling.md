# Production-Hardened Cluster Tooling

**Gap analysis reference:** [streamweave-gap-analysis-and-implementation-strategy.md](streamweave-gap-analysis-and-implementation-strategy.md) §7.

**Dependencies:** None for single-process hardening. Mature cluster sharding for cluster-level tooling.

---

## 1. Objective and rationale

**Objective:** Operational features for running the system in production: deployment, config management, health checks, metrics, logging, upgrades, and failure procedures.

**Why it matters:**

- Without these, “production” means “a process that might be hard to observe or operate.”
- Operators need to know if the system is healthy, how it is performing, and how to change config or shut it down gracefully.
- Production hardening is incremental: each new capability (distribution, checkpointing, sharding) should come with the minimal operational hooks needed to run it safely.

---

## 2. Current state in StreamWeave

- **No cluster:** Single process; no cluster membership or coordination.
- **Tooling:** Likely tests and maybe basic observability; no standardized health, metrics, or structured logging in the core.
- **Config:** Typically env vars or ad-hoc; no first-class “config API” or versioning.
- **Shutdown:** `stop()` exists; graceful drain (finish in-flight, then exit) may or may not be fully guaranteed.

---

## 3. Single-process hardening (do first)

### 3.1 Health endpoint

**Goal:** Answer “is this process / graph healthy?”

- **Liveness:** “I am running” (e.g. process responds to a simple HTTP GET or a file exists). Use for Kubernetes liveness probe.
- **Readiness:** “I am ready to accept work” (e.g. graph has started, all nodes are running, no fatal error). Use for Kubernetes readiness probe; do not send traffic until ready.

**Implementation:**

- Optional **health server** (e.g. a tiny HTTP server on a configurable port, or a callback that the user provides). Endpoints:
  - `GET /live` → 200 if process is alive.
  - `GET /ready` → 200 if graph is started and not in failed state; 503 otherwise.
- Alternatively, expose a **function** `is_ready(&self) -> bool` and let the user plug it into their own HTTP server or health system.

### 3.2 Structured logging

- Use a **structured logging** facade (e.g. `tracing`) so that log events include:
  - Timestamp, level, target (module).
  - Context: graph_id, node_id, port, optional request_id or trace_id.
- Key events: graph start/stop, node start/fail/restart, backpressure, errors. Avoid logging every message (high volume); log summaries or samples if needed.
- Allow the application to set the log level and sink (e.g. env `RUST_LOG`, or a callback).

### 3.3 Metrics

**Goal:** Expose throughput, latency, and error rate per node or per port so that operators can monitor and alert.

**Suggested metrics (names and types are illustrative):**

| Metric | Type | Description |
|--------|------|-------------|
| `streamweave_items_in_total` | Counter | Items received on an input port (or sent on an output port). |
| `streamweave_items_out_total` | Counter | Items sent on an output port. |
| `streamweave_errors_total` | Counter | Errors (e.g. node execute error) per node. |
| `streamweave_channel_backpressure` | Gauge | Current channel length or “waiting” count (if available). |
| `streamweave_progress_frontier` | Gauge | Current completed logical time (if progress tracking is enabled). |

**Integration:** Prefer **OpenTelemetry** or **Prometheus**-compatible exposition so that StreamWeave fits into existing observability stacks. Expose a single endpoint (e.g. `/metrics`) or push to a collector; do not build a full dashboard in the library.

### 3.4 Config management

- **Now:** Support **configuration as data** (e.g. a struct or config file) that is passed at graph build time or at start. Document the format (e.g. TOML/JSON) and which options affect which components.
- **Versioning:** Include a **config version** or **schema version** so that upgrades can detect incompatible config and fail fast or migrate.
- **Sources:** Prefer env vars and files; optional integration with a simple config service later. Avoid hardcoding secrets; support “config from env” or “config from file path.”

### 3.5 Graceful shutdown

- **Drain in-flight:** When the user calls `stop()`, signal all nodes to stop (existing `stop_signal`), then **wait** for in-flight messages to be consumed (or dropped) and for all node tasks to complete. Only then return from `stop()`.
- **Timeout:** Optionally, a **shutdown timeout**: if drain does not complete within N seconds, force-close channels and log a warning.
- **Order:** Document the order of operations (e.g. stop sources first, then downstream, then close channels) so that no task is left waiting forever.

### 3.6 Deployment and upgrades

- **Document:** Recommend deployment patterns (e.g. single binary, version in binary or in config, rollback by redeploying previous version). Do not build a PaaS; prefer integration with **Kubernetes**, **systemd**, or existing deployment tools.
- **Rollback-friendly:** Design so that config and binary version can be rolled back without data loss (where possible); state in external stores should be compatible or migrated.

---

## 4. Cluster-level tooling (with distribution)

When you have a distributed runtime (workers, sharding), add:

- **Cluster health:** e.g. “all shards running,” “quorum reached.” Expose via the same health endpoint or a separate “cluster” endpoint.
- **Config distribution:** e.g. config pushed from a coordinator or read from a shared store so that all workers see the same config.
- **Graceful shutdown:** Coordinate drain across workers (e.g. stop accepting new work, drain in-flight, then exit).
- **Failure procedures:** Document what to do when a worker dies (restart, rebalance) and how to inspect state (e.g. checkpoint location, last committed offset).

---

## 5. What to avoid

- **Building a full PaaS** inside StreamWeave (orchestration, scheduling, service discovery). Prefer integration with Kubernetes, systemd, or external tools.
- **Proprietary metrics format:** Prefer OpenTelemetry/Prometheus so that existing dashboards and alerts work.
- **Logging every message:** Use sampling or aggregate metrics for high-volume streams.

---

## 6. Implementation phases

| Phase | Content |
|-------|--------|
| **1** | Add readiness/liveness semantics: document or implement `is_ready()`, optional HTTP health endpoints. |
| **2** | Integrate structured logging (e.g. `tracing`) for key events (start, stop, error, restart). |
| **3** | Add metrics (counters/gauges) for items in/out, errors, optional backpressure; expose in Prometheus or OTel format. |
| **4** | Document config format and graceful shutdown behavior; add shutdown timeout if needed. |
| **5** | (With distribution) Cluster health and config distribution. |

---

## 7. References

- Gap analysis §7 (Production-hardened cluster tooling).
- OpenTelemetry, Prometheus: metrics and tracing.
- Kubernetes: liveness/readiness probes, deployment patterns.

//! Optional sidecar file (`*.streamweave.yaml`) for graph I/O and metadata.
//!
//! When a `.mmd` file has a co-located sidecar (e.g. `pipeline.streamweave.yaml` next to
//! `pipeline.mmd`), the parser can merge it into the blueprint and the exporter can write it.
//! See the convention doc, section "Optional sidecar file".

use crate::mermaid::blueprint::GraphBlueprint;
use std::path::Path;

#[cfg(feature = "mermaid")]
use crate::mermaid::blueprint::{
  ExecutionMode, InputBinding, NodeInfo, NodeSupervision, OutputBinding, ShardConfig,
};
#[cfg(feature = "mermaid")]
use std::collections::HashMap;

#[cfg(feature = "mermaid")]
use serde::{Deserialize, Serialize};

/// Error when reading or writing a sidecar file.
#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
  /// I/O error (file not found, permission, or unsupported when `mermaid` is off).
  #[error("io error: {0}")]
  Io(#[from] std::io::Error),
  /// Invalid YAML in the sidecar file.
  #[cfg(feature = "mermaid")]
  #[error("yaml error: {0}")]
  Yaml(#[from] serde_yaml::Error),
}

/// Opaque sidecar data (only meaningful when the `mermaid` feature is enabled).
#[cfg(not(feature = "mermaid"))]
pub struct SidecarYaml;

// DTOs for YAML (stable schema; blueprint types stay independent of serde)

#[cfg(feature = "mermaid")]
/// YAML DTO for an input binding (external name → node.port).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct InputBindingYaml {
  /// External input name (e.g. from CLI or API).
  external_name: String,
  /// Target node id.
  node_id: String,
  /// Target input port name on that node.
  port_name: String,
}

#[cfg(feature = "mermaid")]
/// YAML DTO for an output binding (node.port → external name).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct OutputBindingYaml {
  /// External output name.
  external_name: String,
  /// Source node id.
  node_id: String,
  /// Source output port name.
  port_name: String,
}

#[cfg(feature = "mermaid")]
/// YAML DTO for shard configuration (horizontal partitioning).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct ShardConfigYaml {
  /// This worker's shard index (0-based).
  shard_id: u32,
  /// Total number of shards.
  total_shards: u32,
}

#[cfg(feature = "mermaid")]
/// YAML DTO for per-node sidecar overrides (kind, label, supervision).
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
struct NodeSidecarYaml {
  /// Node kind (e.g. MapNode).
  kind: Option<String>,
  /// Display label.
  label: Option<String>,
  /// Supervision policy (e.g. Restart).
  supervision_policy: Option<String>,
  /// Supervision group id for co-restart.
  supervision_group: Option<String>,
}

/// Sidecar data loaded from or to be written to a `*.streamweave.yaml` file.
#[cfg(feature = "mermaid")]
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct SidecarYaml {
  /// Graph name.
  name: Option<String>,
  /// Input bindings (external name → node.port).
  inputs: Option<Vec<InputBindingYaml>>,
  /// Output bindings (node.port → external name).
  outputs: Option<Vec<OutputBindingYaml>>,
  /// Execution mode: `"concurrent"` or `"deterministic"`.
  execution_mode: Option<String>,
  /// Shard configuration when running partitioned.
  shard_config: Option<ShardConfigYaml>,
  /// Per-node overrides (kind, label, supervision).
  nodes: Option<HashMap<String, NodeSidecarYaml>>,
  /// Subgraph unit ids for nested blueprints.
  subgraph_units: Option<Vec<String>>,
  /// Edge ids that are feedback edges (cycle-breaking).
  feedback_edge_ids: Option<Vec<String>>,
}

/// Loads a sidecar file from the given path.
///
/// Returns an error if the file does not exist, is not readable, or is invalid YAML.
/// When the `mermaid` feature is disabled, always returns an error (sidecar requires serde_yaml).
#[cfg(feature = "mermaid")]
pub fn load_sidecar(path: &Path) -> Result<SidecarYaml, SidecarError> {
  let s = std::fs::read_to_string(path)?;
  let sidecar: SidecarYaml = serde_yaml::from_str(&s)?;
  Ok(sidecar)
}

/// Loads a sidecar file (no-op when the `mermaid` feature is disabled).
#[cfg(not(feature = "mermaid"))]
pub fn load_sidecar(_path: &Path) -> Result<SidecarYaml, SidecarError> {
  Err(SidecarError::Io(std::io::Error::new(
    std::io::ErrorKind::Unsupported,
    "sidecar requires the mermaid feature (serde_yaml)",
  )))
}

/// Applies sidecar data into the blueprint (sidecar overrides existing fields when present).
#[cfg(feature = "mermaid")]
pub fn apply_sidecar_to_blueprint(bp: &mut GraphBlueprint, sidecar: &SidecarYaml) {
  if let Some(name) = &sidecar.name {
    bp.name = name.clone();
  }
  if let Some(inputs) = &sidecar.inputs {
    bp.inputs = inputs
      .iter()
      .map(|b| InputBinding {
        external_name: b.external_name.clone(),
        node_id: b.node_id.clone(),
        port_name: b.port_name.clone(),
      })
      .collect();
  }
  if let Some(outputs) = &sidecar.outputs {
    bp.outputs = outputs
      .iter()
      .map(|b| OutputBinding {
        external_name: b.external_name.clone(),
        node_id: b.node_id.clone(),
        port_name: b.port_name.clone(),
      })
      .collect();
  }
  if let Some(mode) = &sidecar.execution_mode {
    bp.execution_mode = if mode.as_str() == "deterministic" {
      ExecutionMode::Deterministic
    } else {
      ExecutionMode::Concurrent
    };
  }
  if let Some(sc) = &sidecar.shard_config {
    bp.shard_config = Some(ShardConfig {
      shard_id: sc.shard_id,
      total_shards: sc.total_shards,
    });
  }
  if let Some(nodes) = &sidecar.nodes {
    for (node_id, n) in nodes {
      if let Some(info) = bp.nodes.get_mut(node_id) {
        if n.kind.is_some() {
          info.kind = n.kind.clone();
        }
        if n.label.is_some() {
          info.label = n.label.clone();
        }
      } else {
        bp.nodes.insert(
          node_id.clone(),
          NodeInfo {
            kind: n.kind.clone(),
            label: n.label.clone(),
          },
        );
      }
      if n.supervision_policy.is_some() || n.supervision_group.is_some() {
        bp.node_supervision.insert(
          node_id.clone(),
          NodeSupervision {
            policy: n
              .supervision_policy
              .clone()
              .unwrap_or_else(|| "Restart".to_string()),
            supervision_group: n.supervision_group.clone(),
          },
        );
      }
    }
  }
  if let Some(units) = &sidecar.subgraph_units {
    bp.subgraph_units = units.clone();
  }
  if let Some(ids) = &sidecar.feedback_edge_ids {
    bp.feedback_edge_ids = ids.clone();
  }
}

/// Writes the blueprint's I/O and metadata to a sidecar file at the given path.
///
/// When the `mermaid` feature is disabled, always returns an error.
#[cfg(feature = "mermaid")]
pub fn write_sidecar(path: &Path, bp: &GraphBlueprint) -> Result<(), SidecarError> {
  let sidecar = SidecarYaml {
    name: Some(bp.name.clone()),
    inputs: Some(
      bp.inputs
        .iter()
        .map(|b| InputBindingYaml {
          external_name: b.external_name.clone(),
          node_id: b.node_id.clone(),
          port_name: b.port_name.clone(),
        })
        .collect(),
    ),
    outputs: Some(
      bp.outputs
        .iter()
        .map(|b| OutputBindingYaml {
          external_name: b.external_name.clone(),
          node_id: b.node_id.clone(),
          port_name: b.port_name.clone(),
        })
        .collect(),
    ),
    execution_mode: Some(match bp.execution_mode {
      ExecutionMode::Concurrent => "concurrent".to_string(),
      ExecutionMode::Deterministic => "deterministic".to_string(),
    }),
    shard_config: bp.shard_config.map(|s| ShardConfigYaml {
      shard_id: s.shard_id,
      total_shards: s.total_shards,
    }),
    nodes: {
      let mut m = HashMap::new();
      for (node_id, info) in &bp.nodes {
        let sup = bp.node_supervision.get(node_id);
        if info.kind.is_some() || info.label.is_some() || sup.is_some() {
          m.insert(
            node_id.clone(),
            NodeSidecarYaml {
              kind: info.kind.clone(),
              label: info.label.clone(),
              supervision_policy: sup.map(|s| s.policy.clone()),
              supervision_group: sup.and_then(|s| s.supervision_group.clone()),
            },
          );
        }
      }
      if m.is_empty() { None } else { Some(m) }
    },
    subgraph_units: if bp.subgraph_units.is_empty() {
      None
    } else {
      Some(bp.subgraph_units.clone())
    },
    feedback_edge_ids: if bp.feedback_edge_ids.is_empty() {
      None
    } else {
      Some(bp.feedback_edge_ids.clone())
    },
  };
  let s = serde_yaml::to_string(&sidecar)?;
  std::fs::write(path, s)?;
  Ok(())
}

/// No-op when the `mermaid` feature is disabled (sidecar data is not parsed).
#[cfg(not(feature = "mermaid"))]
pub fn apply_sidecar_to_blueprint(_bp: &mut GraphBlueprint, _sidecar: &SidecarYaml) {}

/// Returns an error when the `mermaid` feature is disabled (sidecar writing requires serde_yaml).
#[cfg(not(feature = "mermaid"))]
pub fn write_sidecar(_path: &Path, _bp: &GraphBlueprint) -> Result<(), SidecarError> {
  Err(SidecarError::Io(std::io::Error::new(
    std::io::ErrorKind::Unsupported,
    "sidecar requires the mermaid feature (serde_yaml)",
  )))
}

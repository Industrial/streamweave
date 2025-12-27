//! # Health Checks
//!
//! This module provides health check functionality for StreamWeave pipelines,
//! enabling integration with orchestrators like Kubernetes.
//!
//! ## Features
//!
//! - **Health Status**: Track overall pipeline health
//! - **Component Health**: Individual component health tracking
//! - **Liveness/Readiness**: Support for Kubernetes probes
//! - **Health Aggregation**: Combine multiple component healths
//!
//! ## Example
//!
//! ```rust,no_run
//! use streamweave::metrics::health::{HealthCheck, HealthStatus, ComponentHealth};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let mut health_check = HealthCheck::new("my-pipeline");
//!
//! // Register component health
//! health_check.set_component_health("database", ComponentHealth::healthy());
//! health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
//!
//! // Check overall health
//! let status = health_check.status();
//! println!("Pipeline health: {:?}", status);
//!
//! // Use for Kubernetes probes
//! let is_ready = health_check.is_ready();
//! let is_live = health_check.is_live();
//! # Ok(())
//! # }
//! ```

use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::SystemTime;

/// Health status of a component or system.
///
/// Health status indicates the operational state of a component,
/// which can be used for liveness and readiness probes.
///
/// # Example
///
/// ```rust
/// use streamweave::metrics::health::HealthStatus;
///
/// let healthy = HealthStatus::Healthy;
/// let degraded = HealthStatus::Degraded {
///     reason: "High latency detected".to_string(),
/// };
/// let unhealthy = HealthStatus::Unhealthy {
///     reason: "Connection failed".to_string(),
/// };
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HealthStatus {
  /// Component is healthy and operating normally.
  Healthy,
  /// Component is operational but experiencing issues.
  Degraded {
    /// Reason for degraded status.
    reason: String,
  },
  /// Component is not operational.
  Unhealthy {
    /// Reason for unhealthy status.
    reason: String,
  },
}

impl HealthStatus {
  /// Checks if the status indicates the component is healthy.
  ///
  /// # Returns
  ///
  /// `true` if the status is `Healthy`, `false` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::HealthStatus;
  ///
  /// let status = HealthStatus::Healthy;
  /// assert!(status.is_healthy());
  /// ```
  #[must_use]
  pub fn is_healthy(&self) -> bool {
    matches!(self, Self::Healthy)
  }

  /// Checks if the status indicates the component is ready (healthy or degraded).
  ///
  /// # Returns
  ///
  /// `true` if the status is `Healthy` or `Degraded`, `false` if `Unhealthy`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::HealthStatus;
  ///
  /// let healthy = HealthStatus::Healthy;
  /// let degraded = HealthStatus::Degraded {
  ///     reason: "High latency".to_string(),
  /// };
  /// let unhealthy = HealthStatus::Unhealthy {
  ///     reason: "Connection failed".to_string(),
  /// };
  ///
  /// assert!(healthy.is_ready());
  /// assert!(degraded.is_ready());
  /// assert!(!unhealthy.is_ready());
  /// ```
  #[must_use]
  pub fn is_ready(&self) -> bool {
    !matches!(self, Self::Unhealthy { .. })
  }

  /// Gets the reason for the status, if available.
  ///
  /// # Returns
  ///
  /// `Some(reason)` for `Degraded` or `Unhealthy` statuses, `None` for `Healthy`.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::HealthStatus;
  ///
  /// let degraded = HealthStatus::Degraded {
  ///     reason: "High latency".to_string(),
  /// };
  /// assert_eq!(degraded.reason(), Some("High latency"));
  /// ```
  #[must_use]
  pub fn reason(&self) -> Option<&str> {
    match self {
      Self::Healthy => None,
      Self::Degraded { reason } | Self::Unhealthy { reason } => Some(reason),
    }
  }
}

/// Health status of an individual component.
///
/// Component health tracks the status of a specific component
/// within a pipeline, including when it was last checked.
///
/// # Example
///
/// ```rust
/// use streamweave::metrics::health::ComponentHealth;
///
/// let healthy = ComponentHealth::healthy();
/// let degraded = ComponentHealth::degraded("High latency");
/// let unhealthy = ComponentHealth::unhealthy("Connection failed");
/// ```
#[derive(Debug, Clone)]
pub struct ComponentHealth {
  /// The health status of the component.
  status: HealthStatus,
  /// Timestamp when the health was last updated.
  last_updated: SystemTime,
  /// Optional message providing additional context.
  message: Option<String>,
}

impl ComponentHealth {
  /// Creates a healthy component health.
  ///
  /// # Returns
  ///
  /// A `ComponentHealth` with `Healthy` status.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::ComponentHealth;
  ///
  /// let health = ComponentHealth::healthy();
  /// assert!(health.status().is_healthy());
  /// ```
  #[must_use]
  pub fn healthy() -> Self {
    Self {
      status: HealthStatus::Healthy,
      last_updated: SystemTime::now(),
      message: None,
    }
  }

  /// Creates a degraded component health.
  ///
  /// # Arguments
  ///
  /// * `reason` - Reason for the degraded status.
  ///
  /// # Returns
  ///
  /// A `ComponentHealth` with `Degraded` status.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::ComponentHealth;
  ///
  /// let health = ComponentHealth::degraded("High latency detected");
  /// assert!(!health.status().is_healthy());
  /// assert!(health.status().is_ready());
  /// ```
  #[must_use]
  pub fn degraded(reason: impl Into<String>) -> Self {
    Self {
      status: HealthStatus::Degraded {
        reason: reason.into(),
      },
      last_updated: SystemTime::now(),
      message: None,
    }
  }

  /// Creates an unhealthy component health.
  ///
  /// # Arguments
  ///
  /// * `reason` - Reason for the unhealthy status.
  ///
  /// # Returns
  ///
  /// A `ComponentHealth` with `Unhealthy` status.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::ComponentHealth;
  ///
  /// let health = ComponentHealth::unhealthy("Connection failed");
  /// assert!(!health.status().is_healthy());
  /// assert!(!health.status().is_ready());
  /// ```
  #[must_use]
  pub fn unhealthy(reason: impl Into<String>) -> Self {
    Self {
      status: HealthStatus::Unhealthy {
        reason: reason.into(),
      },
      last_updated: SystemTime::now(),
      message: None,
    }
  }

  /// Gets the health status.
  ///
  /// # Returns
  ///
  /// The current health status.
  #[must_use]
  pub fn status(&self) -> &HealthStatus {
    &self.status
  }

  /// Gets the timestamp when health was last updated.
  ///
  /// # Returns
  ///
  /// The last update timestamp.
  #[must_use]
  pub fn last_updated(&self) -> SystemTime {
    self.last_updated
  }

  /// Gets the optional message.
  ///
  /// # Returns
  ///
  /// The message, if any.
  #[must_use]
  pub fn message(&self) -> Option<&str> {
    self.message.as_deref()
  }

  /// Sets an optional message.
  ///
  /// # Arguments
  ///
  /// * `message` - The message to set.
  ///
  /// # Returns
  ///
  /// Self for method chaining.
  #[must_use]
  pub fn with_message(mut self, message: impl Into<String>) -> Self {
    self.message = Some(message.into());
    self
  }
}

/// Health check for a pipeline or system.
///
/// The health check tracks the overall health of a pipeline
/// by aggregating the health of individual components.
///
/// # Example
///
/// ```rust,no_run
/// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let mut health_check = HealthCheck::new("my-pipeline");
///
/// // Register component health
/// health_check.set_component_health("database", ComponentHealth::healthy());
/// health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
///
/// // Check overall health
/// let status = health_check.status();
/// println!("Pipeline health: {:?}", status);
///
/// // Use for Kubernetes probes
/// let is_ready = health_check.is_ready();
/// let is_live = health_check.is_live();
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HealthCheck {
  /// Name of the pipeline or system.
  name: String,
  /// Health status of individual components.
  components: Arc<RwLock<HashMap<String, ComponentHealth>>>,
  /// Overall health status.
  overall_status: Arc<RwLock<HealthStatus>>,
}

impl HealthCheck {
  /// Creates a new health check.
  ///
  /// # Arguments
  ///
  /// * `name` - Name of the pipeline or system.
  ///
  /// # Returns
  ///
  /// A new `HealthCheck` instance with `Healthy` status.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::HealthCheck;
  ///
  /// let health_check = HealthCheck::new("my-pipeline");
  /// assert!(health_check.status().is_healthy());
  /// ```
  #[must_use]
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      components: Arc::new(RwLock::new(HashMap::new())),
      overall_status: Arc::new(RwLock::new(HealthStatus::Healthy)),
    }
  }

  /// Sets the health status of a component.
  ///
  /// # Arguments
  ///
  /// * `component_name` - Name of the component.
  /// * `health` - Health status of the component.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  /// health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
  /// ```
  pub fn set_component_health(&self, component_name: impl Into<String>, health: ComponentHealth) {
    let component_name = component_name.into();
    {
      let mut components = self.components.write().unwrap();
      components.insert(component_name.clone(), health);
    }
    self.update_overall_status();
  }

  /// Gets the health status of a component.
  ///
  /// # Arguments
  ///
  /// * `component_name` - Name of the component.
  ///
  /// # Returns
  ///
  /// `Some(ComponentHealth)` if the component exists, `None` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  ///
  /// let db_health = health_check.get_component_health("database");
  /// assert!(db_health.is_some());
  /// assert!(db_health.unwrap().status().is_healthy());
  /// ```
  #[must_use]
  pub fn get_component_health(&self, component_name: &str) -> Option<ComponentHealth> {
    let components = self.components.read().unwrap();
    components.get(component_name).cloned()
  }

  /// Gets all component healths.
  ///
  /// # Returns
  ///
  /// A map of component names to their health statuses.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  /// health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
  ///
  /// let all_healths = health_check.get_all_component_healths();
  /// assert_eq!(all_healths.len(), 2);
  /// ```
  #[must_use]
  pub fn get_all_component_healths(&self) -> HashMap<String, ComponentHealth> {
    let components = self.components.read().unwrap();
    components.clone()
  }

  /// Gets the overall health status.
  ///
  /// The overall status is computed by aggregating all component healths:
  /// - If any component is `Unhealthy`, overall is `Unhealthy`
  /// - Else if any component is `Degraded`, overall is `Degraded`
  /// - Otherwise, overall is `Healthy`
  ///
  /// # Returns
  ///
  /// The overall health status.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  /// health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
  ///
  /// let overall = health_check.status();
  /// assert!(!overall.is_healthy()); // Degraded due to cache
  /// assert!(overall.is_ready()); // But still ready
  /// ```
  #[must_use]
  pub fn status(&self) -> HealthStatus {
    let overall_status = self.overall_status.read().unwrap();
    overall_status.clone()
  }

  /// Checks if the system is ready (for readiness probes).
  ///
  /// A system is ready if it's `Healthy` or `Degraded`, but not `Unhealthy`.
  ///
  /// # Returns
  ///
  /// `true` if the system is ready, `false` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  ///
  /// assert!(health_check.is_ready());
  /// ```
  #[must_use]
  pub fn is_ready(&self) -> bool {
    self.status().is_ready()
  }

  /// Checks if the system is live (for liveness probes).
  ///
  /// A system is live if it's not completely `Unhealthy`.
  /// This is typically the same as `is_ready()` but can be customized.
  ///
  /// # Returns
  ///
  /// `true` if the system is live, `false` otherwise.
  ///
  /// # Example
  ///
  /// ```rust
  /// use streamweave::metrics::health::{HealthCheck, ComponentHealth};
  ///
  /// let mut health_check = HealthCheck::new("my-pipeline");
  /// health_check.set_component_health("database", ComponentHealth::healthy());
  ///
  /// assert!(health_check.is_live());
  /// ```
  #[must_use]
  pub fn is_live(&self) -> bool {
    !matches!(self.status(), HealthStatus::Unhealthy { .. })
  }

  /// Gets the name of the pipeline or system.
  ///
  /// # Returns
  ///
  /// The name.
  #[must_use]
  pub fn name(&self) -> &str {
    &self.name
  }

  /// Updates the overall health status based on component healths.
  fn update_overall_status(&self) {
    let components = self.components.read().unwrap();

    // Aggregate component healths
    let mut has_unhealthy = false;
    let mut has_degraded = false;
    let mut unhealthy_reasons = Vec::new();
    let mut degraded_reasons = Vec::new();

    for health in components.values() {
      match health.status() {
        HealthStatus::Unhealthy { reason } => {
          has_unhealthy = true;
          unhealthy_reasons.push(reason.clone());
        }
        HealthStatus::Degraded { reason } => {
          has_degraded = true;
          degraded_reasons.push(reason.clone());
        }
        HealthStatus::Healthy => {}
      }
    }

    let new_status = if has_unhealthy {
      HealthStatus::Unhealthy {
        reason: format!("Unhealthy components: {}", unhealthy_reasons.join(", ")),
      }
    } else if has_degraded {
      HealthStatus::Degraded {
        reason: format!("Degraded components: {}", degraded_reasons.join(", ")),
      }
    } else {
      HealthStatus::Healthy
    };

    let mut overall_status = self.overall_status.write().unwrap();
    *overall_status = new_status;
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_health_status() {
    let healthy = HealthStatus::Healthy;
    assert!(healthy.is_healthy());
    assert!(healthy.is_ready());
    assert_eq!(healthy.reason(), None);

    let degraded = HealthStatus::Degraded {
      reason: "High latency".to_string(),
    };
    assert!(!degraded.is_healthy());
    assert!(degraded.is_ready());
    assert_eq!(degraded.reason(), Some("High latency"));

    let unhealthy = HealthStatus::Unhealthy {
      reason: "Connection failed".to_string(),
    };
    assert!(!unhealthy.is_healthy());
    assert!(!unhealthy.is_ready());
    assert_eq!(unhealthy.reason(), Some("Connection failed"));
  }

  #[test]
  fn test_component_health() {
    let healthy = ComponentHealth::healthy();
    assert!(healthy.status().is_healthy());

    let degraded = ComponentHealth::degraded("High latency");
    assert!(!degraded.status().is_healthy());
    assert!(degraded.status().is_ready());

    let unhealthy = ComponentHealth::unhealthy("Connection failed");
    assert!(!unhealthy.status().is_healthy());
    assert!(!unhealthy.status().is_ready());
  }

  #[test]
  fn test_health_check() {
    let health_check = HealthCheck::new("test-pipeline");
    assert!(health_check.status().is_healthy());
    assert!(health_check.is_ready());
    assert!(health_check.is_live());
  }

  #[test]
  fn test_health_check_with_components() {
    let health_check = HealthCheck::new("test-pipeline");

    health_check.set_component_health("database", ComponentHealth::healthy());
    health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));

    let status = health_check.status();
    assert!(!status.is_healthy());
    assert!(status.is_ready());
    assert!(status.reason().is_some());
  }

  #[test]
  fn test_health_check_unhealthy() {
    let health_check = HealthCheck::new("test-pipeline");

    health_check.set_component_health("database", ComponentHealth::healthy());
    health_check.set_component_health("cache", ComponentHealth::unhealthy("Connection failed"));

    let status = health_check.status();
    assert!(!status.is_healthy());
    assert!(!status.is_ready());
    assert!(!health_check.is_live());
    assert!(status.reason().is_some());
  }

  #[test]
  fn test_get_component_health() {
    let health_check = HealthCheck::new("test-pipeline");

    health_check.set_component_health("database", ComponentHealth::healthy());
    let db_health = health_check.get_component_health("database");
    assert!(db_health.is_some());
    assert!(db_health.unwrap().status().is_healthy());

    let missing = health_check.get_component_health("missing");
    assert!(missing.is_none());
  }
}

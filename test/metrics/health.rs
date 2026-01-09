use std::time::SystemTime;
use streamweave_metrics::{ComponentHealth, HealthCheck, HealthStatus};

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
fn test_health_status_equality() {
  let healthy1 = HealthStatus::Healthy;
  let healthy2 = HealthStatus::Healthy;
  assert_eq!(healthy1, healthy2);

  let degraded1 = HealthStatus::Degraded {
    reason: "test".to_string(),
  };
  let degraded2 = HealthStatus::Degraded {
    reason: "test".to_string(),
  };
  assert_eq!(degraded1, degraded2);

  let unhealthy1 = HealthStatus::Unhealthy {
    reason: "test".to_string(),
  };
  let unhealthy2 = HealthStatus::Unhealthy {
    reason: "test".to_string(),
  };
  assert_eq!(unhealthy1, unhealthy2);
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
fn test_component_health_with_message() {
  let mut health = ComponentHealth::healthy();
  assert_eq!(health.message(), None);

  health = health.with_message("Additional context");
  assert_eq!(health.message(), Some("Additional context"));
}

#[test]
fn test_component_health_last_updated() {
  let health = ComponentHealth::healthy();
  let last_updated = health.last_updated();

  // Should be recent (within last second)
  let now = SystemTime::now();
  assert!(now.duration_since(last_updated).unwrap().as_secs() < 1);
}

#[test]
fn test_component_health_clone() {
  let health = ComponentHealth::degraded("test").with_message("msg");
  let cloned = health.clone();

  assert_eq!(health.status(), cloned.status());
  assert_eq!(health.message(), cloned.message());
}

#[test]
fn test_health_check() {
  let health_check = HealthCheck::new("test-pipeline");
  assert!(health_check.status().is_healthy());
  assert!(health_check.is_ready());
  assert!(health_check.is_live());
  assert_eq!(health_check.name(), "test-pipeline");
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

#[test]
fn test_get_all_component_healths() {
  let health_check = HealthCheck::new("test-pipeline");

  health_check.set_component_health("database", ComponentHealth::healthy());
  health_check.set_component_health("cache", ComponentHealth::degraded("High latency"));
  health_check.set_component_health("api", ComponentHealth::unhealthy("Down"));

  let all = health_check.get_all_component_healths();
  assert_eq!(all.len(), 3);
  assert!(all.contains_key("database"));
  assert!(all.contains_key("cache"));
  assert!(all.contains_key("api"));
}

#[test]
fn test_health_check_update_component() {
  let health_check = HealthCheck::new("test-pipeline");

  health_check.set_component_health("database", ComponentHealth::healthy());
  assert!(health_check.status().is_healthy());

  health_check.set_component_health("database", ComponentHealth::degraded("Slow"));
  assert!(!health_check.status().is_healthy());
  assert!(health_check.status().is_ready());
}

#[test]
fn test_health_check_multiple_degraded() {
  let health_check = HealthCheck::new("test-pipeline");

  health_check.set_component_health("cache1", ComponentHealth::degraded("Slow"));
  health_check.set_component_health("cache2", ComponentHealth::degraded("High latency"));

  let status = health_check.status();
  assert!(!status.is_healthy());
  assert!(status.is_ready());
  assert!(status.reason().unwrap().contains("Degraded components"));
}

#[test]
fn test_health_check_multiple_unhealthy() {
  let health_check = HealthCheck::new("test-pipeline");

  health_check.set_component_health("db1", ComponentHealth::unhealthy("Down"));
  health_check.set_component_health("db2", ComponentHealth::unhealthy("Timeout"));

  let status = health_check.status();
  assert!(!status.is_healthy());
  assert!(!status.is_ready());
  assert!(status.reason().unwrap().contains("Unhealthy components"));
}

#[test]
fn test_health_check_mixed_statuses() {
  let health_check = HealthCheck::new("test-pipeline");

  health_check.set_component_health("healthy", ComponentHealth::healthy());
  health_check.set_component_health("degraded", ComponentHealth::degraded("Slow"));
  health_check.set_component_health("unhealthy", ComponentHealth::unhealthy("Down"));

  // Unhealthy should take precedence
  let status = health_check.status();
  assert!(!status.is_healthy());
  assert!(!status.is_ready());
  assert!(!health_check.is_live());
}

#[test]
fn test_health_check_clone() {
  let health_check = HealthCheck::new("test");
  health_check.set_component_health("db", ComponentHealth::healthy());

  let cloned = health_check.clone();
  assert_eq!(cloned.name(), "test");
  assert_eq!(cloned.status(), health_check.status());

  // Both should share state
  cloned.set_component_health("cache", ComponentHealth::degraded("Slow"));
  assert!(!health_check.status().is_healthy());
}

#[test]
fn test_health_check_empty() {
  let health_check = HealthCheck::new("test");
  // No components, should be healthy
  assert!(health_check.status().is_healthy());
  assert!(health_check.get_all_component_healths().is_empty());
}

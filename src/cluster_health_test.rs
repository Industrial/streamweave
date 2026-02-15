//! Tests for cluster health aggregation.

use crate::cluster_health::aggregate_cluster_health;

#[test]
fn test_all_healthy() {
    let r = aggregate_cluster_health(&[true, true, true]);
    assert_eq!(r.healthy_count, 3);
    assert_eq!(r.total, 3);
    assert!(r.all_healthy);
    assert!(r.quorum_healthy);
    assert!(r.is_ready());
}

#[test]
fn test_quorum_but_not_all() {
    let r = aggregate_cluster_health(&[true, true, false]);
    assert_eq!(r.healthy_count, 2);
    assert!(!r.all_healthy);
    assert!(r.quorum_healthy); // 2 >= ceil(3/2)=2
}

#[test]
fn test_no_quorum() {
    let r = aggregate_cluster_health(&[true, false, false]);
    assert_eq!(r.healthy_count, 1);
    assert!(!r.all_healthy);
    assert!(!r.quorum_healthy);
    assert!(!r.is_ready());
}

#[test]
fn test_empty() {
    let r = aggregate_cluster_health(&[]);
    assert_eq!(r.healthy_count, 0);
    assert_eq!(r.total, 0);
    assert!(!r.all_healthy);
    assert!(!r.quorum_healthy);
}

#[test]
fn test_single_shard() {
    let r = aggregate_cluster_health(&[true]);
    assert!(r.all_healthy);
    assert!(r.quorum_healthy);
}

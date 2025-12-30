use streamweave_visualization::{Breakpoint, DebugState, Debugger};

#[tokio::test]
async fn test_breakpoint_creation() {
  let breakpoint = Breakpoint::at_node("node_1".to_string());
  assert_eq!(breakpoint.node_id, "node_1");
  assert!(breakpoint.enabled);
  assert!(breakpoint.condition.is_none());
}

#[tokio::test]
async fn test_breakpoint_with_condition() {
  let breakpoint = Breakpoint::with_condition("node_1".to_string(), "item.value > 100".to_string());
  assert_eq!(breakpoint.node_id, "node_1");
  assert_eq!(breakpoint.condition, Some("item.value > 100".to_string()));
}

#[tokio::test]
async fn test_debugger_creation() {
  let debugger = Debugger::new();
  let state = debugger.get_state().await;
  assert_eq!(state, DebugState::Running);
}

#[tokio::test]
async fn test_debugger_add_breakpoint() {
  let mut debugger = Debugger::new();
  debugger
    .add_breakpoint(Breakpoint::at_node("node_1".to_string()))
    .await;

  let breakpoints = debugger.breakpoints.read().await;
  assert_eq!(breakpoints.len(), 1);
  assert_eq!(breakpoints[0].node_id, "node_1");
}

#[tokio::test]
async fn test_debugger_should_pause() {
  let mut debugger = Debugger::new();
  debugger
    .add_breakpoint(Breakpoint::at_node("node_1".to_string()))
    .await;

  let should_pause = debugger.should_pause("node_1".to_string(), None).await;
  assert!(should_pause);

  // Resume the debugger so we can test the next node
  debugger.resume().await;

  let should_not_pause = debugger.should_pause("node_2".to_string(), None).await;
  assert!(!should_not_pause);
}

#[tokio::test]
async fn test_debugger_resume() {
  let mut debugger = Debugger::new();
  debugger
    .add_breakpoint(Breakpoint::at_node("node_1".to_string()))
    .await;

  let _ = debugger.should_pause("node_1".to_string(), None).await;
  let state = debugger.get_state().await;
  assert!(matches!(state, DebugState::Paused { .. }));

  debugger.resume().await;
  let state = debugger.get_state().await;
  assert_eq!(state, DebugState::Running);
}

#[tokio::test]
async fn test_debugger_step() {
  let debugger = Debugger::new();
  debugger.step().await;
  let state = debugger.get_state().await;
  assert_eq!(state, DebugState::Stepping);
}

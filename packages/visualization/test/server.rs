use std::net::SocketAddr;
use streamweave_visualization::PipelineDag;
use streamweave_visualization::VisualizationServer;

#[test]
fn test_visualization_server_new() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server = VisualizationServer::new(addr);
  assert_eq!(server.address(), addr);
}

#[test]
fn test_visualization_server_address() {
  let addr: SocketAddr = "0.0.0.0:3000".parse().unwrap();
  let server = VisualizationServer::new(addr);
  assert_eq!(server.address(), addr);
}

#[test]
fn test_visualization_server_clone() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server1 = VisualizationServer::new(addr);
  let server2 = server1.clone();
  assert_eq!(server1.address(), server2.address());
}

#[tokio::test]
async fn test_visualization_server_serve_dag() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server = VisualizationServer::new(addr);
  let dag = PipelineDag::new();

  // This should succeed (placeholder implementation)
  let result = server.serve_dag(dag).await;
  assert!(result.is_ok());
}

#[tokio::test]
async fn test_visualization_server_serve_dag_multiple() {
  let addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();
  let server = VisualizationServer::new(addr);

  let dag1 = PipelineDag::new();
  let dag2 = PipelineDag::new();

  // Both should succeed
  assert!(server.serve_dag(dag1).await.is_ok());
  assert!(server.serve_dag(dag2).await.is_ok());
}

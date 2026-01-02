use futures::{StreamExt, stream};
use proptest::prelude::*;
use std::pin::Pin;
use streamweave::Producer;
use streamweave_graph::control_flow::ErrorBranch;
use streamweave_graph::router::OutputRouter;
use streamweave_vec::VecProducer;

#[tokio::test]
async fn test_error_branch_router_basic() {
  let mut router = ErrorBranch::<i32, String>::new();
  let input_stream: Pin<Box<dyn futures::Stream<Item = Result<i32, String>> + Send>> =
    Box::pin(stream::iter(vec![
      Ok(1),
      Err("error1".to_string()),
      Ok(2),
      Err("error2".to_string()),
      Ok(3),
    ]));

  let mut output_streams = router.route_stream(input_stream).await;
  assert_eq!(output_streams.len(), 2);

  // Collect from success port
  let mut success_results = Vec::new();
  let stream_success = &mut output_streams[0].1;
  while let Some(result) = stream_success.next().await {
    success_results.push(result);
  }

  // Collect from error port
  let mut error_results = Vec::new();
  let stream_error = &mut output_streams[1].1;
  while let Some(result) = stream_error.next().await {
    error_results.push(result);
  }

  assert_eq!(success_results.len(), 3);
  assert_eq!(error_results.len(), 2);

  assert!(success_results[0].is_ok());
  assert!(success_results[1].is_ok());
  assert!(success_results[2].is_ok());

  assert!(error_results[0].is_err());
  assert!(error_results[1].is_err());
}

#[tokio::test]
async fn test_error_branch_router_all_success() {
  let mut router = ErrorBranch::<i32, String>::new();
  let input_stream: Pin<Box<dyn futures::Stream<Item = Result<i32, String>> + Send>> =
    Box::pin(stream::iter(vec![Ok(1), Ok(2), Ok(3)]));

  let mut output_streams = router.route_stream(input_stream).await;

  let mut success_results = Vec::new();
  let stream_success = &mut output_streams[0].1;
  while let Some(result) = stream_success.next().await {
    success_results.push(result);
  }

  assert_eq!(success_results.len(), 3);
}

#[tokio::test]
async fn test_error_branch_router_all_errors() {
  let mut router = ErrorBranch::<i32, String>::new();
  let input_stream: Pin<Box<dyn futures::Stream<Item = Result<i32, String>> + Send>> =
    Box::pin(stream::iter(vec![
      Err("error1".to_string()),
      Err("error2".to_string()),
    ]));

  let mut output_streams = router.route_stream(input_stream).await;

  let mut error_results = Vec::new();
  let stream_error = &mut output_streams[1].1;
  while let Some(result) = stream_error.next().await {
    error_results.push(result);
  }

  assert_eq!(error_results.len(), 2);
}

#[test]
fn test_error_branch_router_default() {
  let router1 = ErrorBranch::<i32, String>::new();
  let router2 = ErrorBranch::<i32, String>::default();
  assert_eq!(router1.output_port_names(), router2.output_port_names());
}

#[test]
fn test_error_branch_router_output_ports() {
  let router = ErrorBranch::<i32, String>::new();
  assert_eq!(
    router.output_port_names(),
    vec!["success".to_string(), "error".to_string()]
  );
}

#[tokio::test]
async fn test_error_branch_router() {
  let mut router = ErrorBranch::<i32, String>::new();
  let results: Vec<Result<i32, String>> = vec![
    Ok(1),
    Err("error1".to_string()),
    Ok(2),
    Err("error2".to_string()),
    Ok(3),
  ];

  let mut producer = VecProducer::new(results);
  let stream = producer.produce();

  let mut routed = router.route_stream(Box::pin(stream)).await;
  assert_eq!(routed.len(), 2); // success port and error port

  // Collect from success port
  let success_stream = &mut routed[0].1;
  let mut success_results = Vec::new();
  while let Some(item) = success_stream.next().await {
    success_results.push(item);
  }

  // Collect from error port
  let error_stream = &mut routed[1].1;
  let mut error_results = Vec::new();
  while let Some(item) = error_stream.next().await {
    error_results.push(item);
  }

  assert_eq!(success_results.len(), 3);
  assert_eq!(error_results.len(), 2);
  assert!(success_results.iter().all(|r| r.is_ok()));
  assert!(error_results.iter().all(|r| r.is_err()));
}

#[tokio::test]
async fn test_error_branch_default() {
  let router1 = ErrorBranch::<i32, String>::new();
  let router2 = ErrorBranch::<i32, String>::default();
  assert_eq!(router1.output_port_names(), router2.output_port_names());
}

proptest! {
  #![proptest_config(ProptestConfig::with_cases(10))]

  #[test]
  fn test_error_branch_proptest(items in prop::collection::vec(
    prop::sample::select(vec![
      Ok(0i32),
      Err("error".to_string()),
    ]),
    0..20
  )) {
    let rt = tokio::runtime::Builder::new_current_thread()
      .enable_all()
      .build()
      .unwrap();
    rt.block_on(async {
      let mut router = ErrorBranch::<i32, String>::new();
      let input_stream: Pin<Box<dyn futures::Stream<Item = Result<i32, String>> + Send>> =
        Box::pin(stream::iter(items.clone()));

      let mut output_streams = router.route_stream(input_stream).await;
      assert_eq!(output_streams.len(), 2);

      // Collect from success port
      let success_stream = &mut output_streams[0].1;
      let mut success_results = Vec::new();
      while let Some(item) = success_stream.next().await {
        success_results.push(item);
      }

      // Collect from error port
      let error_stream = &mut output_streams[1].1;
      let mut error_results = Vec::new();
      while let Some(item) = error_stream.next().await {
        error_results.push(item);
      }

      // Verify all results are correctly routed
      assert_eq!(success_results.len() + error_results.len(), items.len());
      assert!(success_results.iter().all(|r| r.is_ok()));
      assert!(error_results.iter().all(|r| r.is_err()));

      // Verify counts match
      let expected_success = items.iter().filter(|r| r.is_ok()).count();
      let expected_error = items.iter().filter(|r| r.is_err()).count();
      assert_eq!(success_results.len(), expected_success);
      assert_eq!(error_results.len(), expected_error);
    });
  }
}

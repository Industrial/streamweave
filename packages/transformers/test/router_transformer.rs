use futures::StreamExt;
use futures::stream;
use streamweave_transformers::Input;
use streamweave_transformers::Transformer;
use streamweave_transformers::{RouteTarget, RouterTransformer};

#[cfg(test)]
mod tests {
  use super::*;

  #[tokio::test]
  async fn test_router_basic() {
    let mut router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x % 2 == 0 {
        RouteTarget::named("even")
      } else {
        RouteTarget::named("odd")
      }
    });

    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).await.collect().await;

    assert_eq!(result.len(), 5);
    assert_eq!(result[0], (RouteTarget::Named("odd".to_string()), 1));
    assert_eq!(result[1], (RouteTarget::Named("even".to_string()), 2));
  }

  #[tokio::test]
  async fn test_router_with_drop() {
    let mut router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x < 0 {
        RouteTarget::Drop
      } else {
        RouteTarget::named("positive")
      }
    });

    let input = stream::iter(vec![-2, -1, 0, 1, 2]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).await.collect().await;

    // Negative numbers should be dropped
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], (RouteTarget::Named("positive".to_string()), 0));
  }
}

use crate::error::{ComponentInfo, ErrorAction, ErrorContext, ErrorStrategy, StreamError};
use crate::transformer::{Transformer, TransformerConfig};
use crate::transformers::router::router_transformer::{RouteTarget, RouterTransformer};
use async_trait::async_trait;
use futures::StreamExt;

#[async_trait]
impl<T, F> Transformer for RouterTransformer<T, F>
where
  T: std::fmt::Debug + Clone + Send + Sync + 'static,
  F: Fn(&T) -> RouteTarget + Clone + Send + Sync + 'static,
{
  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    let router_fn = self.router_fn.clone();
    let default_target = self.default_target.clone();

    Box::pin(input.filter_map(move |item| {
      let target = router_fn(&item);
      let resolved_target = match target {
        RouteTarget::Default => default_target.clone().unwrap_or(RouteTarget::Default),
        other => other,
      };

      async move {
        match resolved_target {
          RouteTarget::Drop => None, // Filter out dropped elements
          target => Some((target, item)),
        }
      }
    }))
  }

  fn set_config_impl(&mut self, config: TransformerConfig<T>) {
    self.config = config;
  }

  fn get_config_impl(&self) -> &TransformerConfig<T> {
    &self.config
  }

  fn get_config_mut_impl(&mut self) -> &mut TransformerConfig<T> {
    &mut self.config
  }

  fn handle_error(&self, error: &StreamError<T>) -> ErrorAction {
    match &self.config.error_strategy {
      ErrorStrategy::Stop => ErrorAction::Stop,
      ErrorStrategy::Skip => ErrorAction::Skip,
      ErrorStrategy::Retry(n) if error.retries < *n => ErrorAction::Retry,
      _ => ErrorAction::Stop,
    }
  }

  fn create_error_context(&self, item: Option<T>) -> ErrorContext<T> {
    ErrorContext {
      timestamp: chrono::Utc::now(),
      item,
      component_name: self.component_info().name,
      component_type: std::any::type_name::<Self>().to_string(),
    }
  }

  fn component_info(&self) -> ComponentInfo {
    ComponentInfo {
      name: self
        .config
        .name
        .clone()
        .unwrap_or_else(|| "router_transformer".to_string()),
      type_name: std::any::type_name::<Self>().to_string(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;

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

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 5);
    assert_eq!(result[0], (RouteTarget::Named("odd".to_string()), 1));
    assert_eq!(result[1], (RouteTarget::Named("even".to_string()), 2));
    assert_eq!(result[2], (RouteTarget::Named("odd".to_string()), 3));
    assert_eq!(result[3], (RouteTarget::Named("even".to_string()), 4));
    assert_eq!(result[4], (RouteTarget::Named("odd".to_string()), 5));
  }

  #[tokio::test]
  async fn test_router_with_index() {
    let mut router =
      RouterTransformer::<i32, _>::new(|x: &i32| RouteTarget::index((*x as usize) % 3));

    let input = stream::iter(vec![0, 1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 6);
    assert_eq!(result[0], (RouteTarget::Index(0), 0));
    assert_eq!(result[1], (RouteTarget::Index(1), 1));
    assert_eq!(result[2], (RouteTarget::Index(2), 2));
    assert_eq!(result[3], (RouteTarget::Index(0), 3));
    assert_eq!(result[4], (RouteTarget::Index(1), 4));
    assert_eq!(result[5], (RouteTarget::Index(2), 5));
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

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    // Negative numbers should be dropped
    assert_eq!(result.len(), 3);
    assert_eq!(result[0], (RouteTarget::Named("positive".to_string()), 0));
    assert_eq!(result[1], (RouteTarget::Named("positive".to_string()), 1));
    assert_eq!(result[2], (RouteTarget::Named("positive".to_string()), 2));
  }

  #[tokio::test]
  async fn test_router_with_default_target() {
    let mut router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x > 10 {
        RouteTarget::named("high")
      } else {
        RouteTarget::Default
      }
    })
    .with_default_target(RouteTarget::named("normal"));

    let input = stream::iter(vec![5, 15, 8, 20]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 4);
    assert_eq!(result[0], (RouteTarget::Named("normal".to_string()), 5));
    assert_eq!(result[1], (RouteTarget::Named("high".to_string()), 15));
    assert_eq!(result[2], (RouteTarget::Named("normal".to_string()), 8));
    assert_eq!(result[3], (RouteTarget::Named("high".to_string()), 20));
  }

  #[tokio::test]
  async fn test_router_empty_input() {
    let mut router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"));

    let input = stream::iter(Vec::<i32>::new());
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[tokio::test]
  async fn test_router_with_strings() {
    let mut router = RouterTransformer::<String, _>::new(|s: &String| {
      if s.starts_with("ERR") {
        RouteTarget::named("errors")
      } else if s.starts_with("WARN") {
        RouteTarget::named("warnings")
      } else {
        RouteTarget::named("info")
      }
    });

    let input = stream::iter(vec![
      "ERR: Failed".to_string(),
      "INFO: OK".to_string(),
      "WARN: Low".to_string(),
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, String)> = router.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(
      result[0],
      (
        RouteTarget::Named("errors".to_string()),
        "ERR: Failed".to_string()
      )
    );
    assert_eq!(
      result[1],
      (
        RouteTarget::Named("info".to_string()),
        "INFO: OK".to_string()
      )
    );
    assert_eq!(
      result[2],
      (
        RouteTarget::Named("warnings".to_string()),
        "WARN: Low".to_string()
      )
    );
  }

  #[tokio::test]
  async fn test_router_all_dropped() {
    let mut router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::Drop);

    let input = stream::iter(vec![1, 2, 3, 4, 5]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    assert!(result.is_empty());
  }

  #[test]
  fn test_router_component_info() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"))
      .with_name("my_router".to_string());

    let info = router.component_info();
    assert_eq!(info.name, "my_router");
    assert!(info.type_name.contains("RouterTransformer"));
  }

  #[test]
  fn test_router_default_component_info() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"));

    let info = router.component_info();
    assert_eq!(info.name, "router_transformer");
  }

  #[test]
  fn test_router_error_handling_stop() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"))
      .with_error_strategy(ErrorStrategy::Stop);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RouterTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RouterTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(router.handle_error(&error), ErrorAction::Stop));
  }

  #[test]
  fn test_router_error_handling_skip() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"))
      .with_error_strategy(ErrorStrategy::Skip);

    let error = StreamError {
      source: Box::new(std::io::Error::other("test error")),
      context: ErrorContext {
        timestamp: chrono::Utc::now(),
        item: None,
        component_name: "test".to_string(),
        component_type: "RouterTransformer".to_string(),
      },
      component: ComponentInfo {
        name: "test".to_string(),
        type_name: "RouterTransformer".to_string(),
      },
      retries: 0,
    };

    assert!(matches!(router.handle_error(&error), ErrorAction::Skip));
  }

  #[test]
  fn test_router_create_error_context() {
    let router = RouterTransformer::<i32, _>::new(|_: &i32| RouteTarget::named("test"))
      .with_name("test_router".to_string());

    let context = router.create_error_context(Some(42));
    assert_eq!(context.component_name, "test_router");
    assert_eq!(context.item, Some(42));
  }

  #[tokio::test]
  async fn test_router_reusability() {
    let mut router = RouterTransformer::<i32, _>::new(|x: &i32| {
      if *x % 2 == 0 {
        RouteTarget::named("even")
      } else {
        RouteTarget::named("odd")
      }
    });

    // First use
    let input1 = stream::iter(vec![1, 2]);
    let result1: Vec<(RouteTarget, i32)> = router.transform(Box::pin(input1)).collect().await;
    assert_eq!(result1.len(), 2);

    // Second use
    let input2 = stream::iter(vec![3, 4]);
    let result2: Vec<(RouteTarget, i32)> = router.transform(Box::pin(input2)).collect().await;
    assert_eq!(result2.len(), 2);
  }

  #[tokio::test]
  async fn test_router_with_complex_type() {
    #[derive(Debug, Clone, PartialEq)]
    struct Event {
      event_type: String,
      payload: i32,
    }

    let mut router = RouterTransformer::<Event, _>::new(|e: &Event| match e.event_type.as_str() {
      "click" => RouteTarget::named("ui_events"),
      "error" => RouteTarget::named("errors"),
      _ => RouteTarget::named("other"),
    });

    let input = stream::iter(vec![
      Event {
        event_type: "click".to_string(),
        payload: 1,
      },
      Event {
        event_type: "error".to_string(),
        payload: 2,
      },
      Event {
        event_type: "load".to_string(),
        payload: 3,
      },
    ]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, Event)> = router.transform(boxed_input).collect().await;

    assert_eq!(result.len(), 3);
    assert_eq!(result[0].0, RouteTarget::Named("ui_events".to_string()));
    assert_eq!(result[0].1.payload, 1);
    assert_eq!(result[1].0, RouteTarget::Named("errors".to_string()));
    assert_eq!(result[1].1.payload, 2);
    assert_eq!(result[2].0, RouteTarget::Named("other".to_string()));
    assert_eq!(result[2].1.payload, 3);
  }

  #[tokio::test]
  async fn test_router_mixed_routing() {
    let mut router = RouterTransformer::<i32, _>::new(|x: &i32| {
      match *x {
        n if n < 0 => RouteTarget::Drop,          // Drop negatives
        0 => RouteTarget::Default,                // Default for zero
        n if n % 2 == 0 => RouteTarget::index(0), // Even to index 0
        _ => RouteTarget::named("odd"),           // Odd to named route
      }
    })
    .with_default_target(RouteTarget::named("zero"));

    let input = stream::iter(vec![-1, 0, 1, 2, 3, 4]);
    let boxed_input = Box::pin(input);

    let result: Vec<(RouteTarget, i32)> = router.transform(boxed_input).collect().await;

    // -1 is dropped, so only 5 elements
    assert_eq!(result.len(), 5);
    assert_eq!(result[0], (RouteTarget::Named("zero".to_string()), 0));
    assert_eq!(result[1], (RouteTarget::Named("odd".to_string()), 1));
    assert_eq!(result[2], (RouteTarget::Index(0), 2));
    assert_eq!(result[3], (RouteTarget::Named("odd".to_string()), 3));
    assert_eq!(result[4], (RouteTarget::Index(0), 4));
  }
}

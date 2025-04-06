use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, stream};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::sync::Arc;

#[derive(Debug)]
pub enum EnvVarError {
  VarError(std::env::VarError),
  StreamError(String),
}

impl fmt::Display for EnvVarError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      EnvVarError::VarError(e) => write!(f, "Environment variable error: {}", e),
      EnvVarError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for EnvVarError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      EnvVarError::VarError(e) => Some(e),
      EnvVarError::StreamError(_) => None,
    }
  }
}

pub struct EnvVarProducer {
  filter: Option<Vec<String>>,
}

impl EnvVarProducer {
  pub fn new() -> Self {
    Self { filter: None }
  }

  pub fn with_vars(vars: Vec<String>) -> Self {
    Self { filter: Some(vars) }
  }
}

impl Default for EnvVarProducer {
  fn default() -> Self {
    Self::new()
  }
}

impl Error for EnvVarProducer {
  type Error = EnvVarError;
}

impl Output for EnvVarProducer {
  type Output = (String, String);
  type OutputStream = Pin<Box<dyn Stream<Item = Result<(String, String), EnvVarError>> + Send>>;
}

impl Producer for EnvVarProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let vars = match &self.filter {
      Some(filter) => {
        let vars: Vec<_> = filter
          .iter()
          .map(|key| {
            std::env::var(key)
              .map(|value| (key.clone(), value))
              .map_err(EnvVarError::VarError)
          })
          .collect();
        Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
      }
      None => {
        let vars: Vec<_> = std::env::vars().map(Ok).collect();
        Box::pin(stream::iter(vars))
      }
    };
    vars
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;
  use std::collections::HashMap;
  use std::env;
  use std::sync::Arc;

  struct TestEnvVarProducer {
    filter: Option<Vec<String>>,
    env_provider: Arc<dyn Fn(&str) -> Option<String> + Send + Sync>,
  }

  impl TestEnvVarProducer {
    fn new() -> Self {
      Self {
        filter: None,
        env_provider: Arc::new(|key| std::env::var(key).ok()),
      }
    }

    fn with_vars(vars: Vec<String>) -> Self {
      Self {
        filter: Some(vars),
        env_provider: Arc::new(|key| std::env::var(key).ok()),
      }
    }

    fn with_mock_env(env_vars: HashMap<String, String>) -> Self {
      let env_vars = Arc::new(env_vars);
      Self {
        filter: None,
        env_provider: Arc::new(move |key| env_vars.get(key).cloned()),
      }
    }
  }

  impl Error for TestEnvVarProducer {
    type Error = EnvVarError;
  }

  impl Output for TestEnvVarProducer {
    type Output = (String, String);
    type OutputStream = Pin<Box<dyn Stream<Item = Result<Self::Output, Self::Error>> + Send>>;
  }

  impl Producer for TestEnvVarProducer {
    fn produce(&mut self) -> Self::OutputStream {
      let env_provider = self.env_provider.clone();
      let vars = match &self.filter {
        Some(filter) => {
          let vars: Vec<_> = filter
            .iter()
            .map(|key| {
              env_provider(key)
                .map(|value| (key.clone(), value))
                .ok_or_else(|| EnvVarError::VarError(std::env::VarError::NotPresent))
            })
            .collect();
          Box::pin(stream::iter(vars)) as Pin<Box<dyn Stream<Item = _> + Send>>
        }
        None => {
          let vars: Vec<_> = std::env::vars()
            .filter(|(key, _)| env_provider(key).is_some())
            .map(Ok)
            .collect();
          Box::pin(stream::iter(vars))
        }
      };
      vars
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_all() {
    unsafe {
      env::set_var("TEST_VAR_1", "value1");
      env::set_var("TEST_VAR_2", "value2");
      env::set_var("TEST_VAR_3", "value3");
    }

    let mut producer = EnvVarProducer::new();
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;

    assert!(
      !result.is_empty(),
      "Expected environment variables, but found none"
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_1" && v == "value1")
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_2" && v == "value2")
    );
    assert!(
      result
        .iter()
        .any(|(k, v)| k == "TEST_VAR_3" && v == "value3")
    );

    unsafe {
      env::remove_var("TEST_VAR_1");
      env::remove_var("TEST_VAR_2");
      env::remove_var("TEST_VAR_3");
    }
  }

  // #[tokio::test]
  // async fn test_env_var_producer_mock_env() {
  //   let mut mock_env = HashMap::new();
  //   mock_env.insert("MOCK_VAR_1".to_string(), "mock1".to_string());
  //   mock_env.insert("MOCK_VAR_2".to_string(), "mock2".to_string());
  //   let mut producer = TestEnvVarProducer::with_mock_env(mock_env);
  //   let stream = producer.produce();
  //   let result: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;
  //   assert_eq!(result.len(), 2);
  //   assert!(
  //     result
  //       .iter()
  //       .any(|(k, v)| k == "MOCK_VAR_1" && v == "mock1")
  //   );
  //   assert!(
  //     result
  //       .iter()
  //       .any(|(k, v)| k == "MOCK_VAR_2" && v == "mock2")
  //   );
  // }

  #[tokio::test]
  async fn test_env_var_producer_filtered() {
    unsafe {
      env::set_var("TEST_FILTER_1", "value1");
      env::set_var("TEST_FILTER_2", "value2");
    }

    let mut producer = EnvVarProducer::with_vars(vec!["TEST_FILTER_1".to_string()]);
    let stream = producer.produce();
    let result: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;

    assert_eq!(result.len(), 1);
    assert_eq!(
      result[0],
      ("TEST_FILTER_1".to_string(), "value1".to_string())
    );

    unsafe {
      env::remove_var("TEST_FILTER_1");
      env::remove_var("TEST_FILTER_2");
    }
  }

  #[tokio::test]
  async fn test_multiple_calls() {
    unsafe {
      env::set_var("TEST_MULTI", "value");
    }

    let mut producer = EnvVarProducer::with_vars(vec!["TEST_MULTI".to_string()]);

    // First call
    let stream = producer.produce();
    let result1: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1[0], ("TEST_MULTI".to_string(), "value".to_string()));

    // Second call
    let stream = producer.produce();
    let result2: Vec<(String, String)> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2[0], ("TEST_MULTI".to_string(), "value".to_string()));

    unsafe {
      env::remove_var("TEST_MULTI");
    }
  }

  #[tokio::test]
  async fn test_env_var_producer_missing_var() {
    let mut producer = EnvVarProducer::with_vars(vec!["NONEXISTENT_VAR".to_string()]);
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(EnvVarError::VarError(_))));
  }
}

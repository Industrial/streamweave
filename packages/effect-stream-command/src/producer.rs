use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use effect_core::error::EffectError;
use effect_stream::{EffectResult, EffectStream, EffectStreamSource};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

use crate::error::CommandStreamError;

/// A producer that reads from a command's output
pub struct CommandProducer {
  command: String,
  args: Vec<String>,
  config: ProducerConfig,
}

/// Configuration for the command producer
#[derive(Clone, Debug)]
pub struct ProducerConfig {
  /// Timeout for command execution
  pub timeout: Option<Duration>,
  /// Working directory for the command
  pub working_dir: Option<String>,
  /// Environment variables for the command
  pub env: Vec<(String, String)>,
  /// Whether to capture stderr
  pub capture_stderr: bool,
}

impl Default for ProducerConfig {
  fn default() -> Self {
    Self {
      timeout: None,
      working_dir: None,
      env: Vec::new(),
      capture_stderr: false,
    }
  }
}

impl CommandProducer {
  /// Create a new command producer
  pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
    Self {
      command: command.into(),
      args,
      config: ProducerConfig::default(),
    }
  }

  /// Set the configuration for this producer
  pub fn with_config(mut self, config: ProducerConfig) -> Self {
    self.config = config;
    self
  }
}

impl EffectStreamSource<String, CommandStreamError> for CommandProducer {
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<String, CommandStreamError>, CommandStreamError>>
        + Send
        + 'static,
    >,
  >;

  fn source(&self) -> Self::Stream {
    let command = self.command.clone();
    let args = self.args.clone();
    let config = self.config.clone();

    Box::pin(async move {
      let mut cmd = Command::new(&command);
      cmd.args(&args);

      if let Some(dir) = &config.working_dir {
        cmd.current_dir(dir);
      }

      for (key, value) in &config.env {
        cmd.env(key, value);
      }

      if config.capture_stderr {
        cmd.stderr(std::process::Stdio::piped());
      }

      let mut child = cmd
        .stdout(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CommandStreamError::CommandStart(e.to_string()))?;

      let stdout = child.stdout.take().unwrap();
      let mut reader = BufReader::new(stdout);
      let mut stream = EffectStream::<String, CommandStreamError>::new();

      tokio::spawn(async move {
        let mut line = String::new();
        while let Ok(n) = reader.read_line(&mut line).await {
          if n == 0 {
            break;
          }

          let value = line.trim().to_string();
          stream.push(value).await.unwrap();
          line.clear();
        }

        let status = child
          .wait()
          .await
          .map_err(|e| CommandStreamError::CommandExecution(e.to_string()))?;

        if !status.success() {
          stream
            .push_error(CommandStreamError::CommandExecution(format!(
              "Command failed with status {}",
              status
            )))
            .await
            .unwrap();
        }

        stream.close().await.unwrap();
      });

      Ok(stream)
    })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use tokio_test::block_on;

  #[test]
  fn test_command_producer() {
    let producer = CommandProducer::new("echo", vec!["test".to_string()]);
    let stream = block_on(producer.source()).unwrap();

    let mut values = Vec::new();
    block_on(async {
      while let Ok(Some(value)) = stream.next().await {
        values.push(value);
      }
    });

    assert_eq!(values, vec!["test"]);
  }

  #[test]
  fn test_command_producer_multiple_lines() {
    let producer = CommandProducer::new(
      "echo",
      vec!["-e".to_string(), "line 1\nline 2\nline 3".to_string()],
    );
    let stream = block_on(producer.source()).unwrap();

    let mut values = Vec::new();
    block_on(async {
      while let Ok(Some(value)) = stream.next().await {
        values.push(value);
      }
    });

    assert_eq!(values, vec!["line 1", "line 2", "line 3"]);
  }

  #[test]
  fn test_command_producer_nonexistent() {
    let producer = CommandProducer::new("nonexistent_command", vec![]);
    let result = block_on(producer.source());

    assert!(matches!(result, Err(CommandStreamError::CommandStart(_))));
  }
}

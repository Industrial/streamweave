use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

use effect_core::error::EffectError;
use effect_stream::{EffectResult, EffectStream, EffectStreamSink};
use tokio::io::{AsyncWriteExt, BufWriter};
use tokio::process::Command;

use crate::error::CommandStreamError;

/// A consumer that writes to a command's input
pub struct CommandConsumer {
  command: String,
  args: Vec<String>,
  config: ConsumerConfig,
}

/// Configuration for the command consumer
#[derive(Clone, Debug)]
pub struct ConsumerConfig {
  /// Timeout for command execution
  pub timeout: Option<Duration>,
  /// Working directory for the command
  pub working_dir: Option<String>,
  /// Environment variables for the command
  pub env: Vec<(String, String)>,
  /// Whether to append newlines to input
  pub append_newlines: bool,
}

impl Default for ConsumerConfig {
  fn default() -> Self {
    Self {
      timeout: None,
      working_dir: None,
      env: Vec::new(),
      append_newlines: true,
    }
  }
}

impl CommandConsumer {
  /// Create a new command consumer
  pub fn new(command: impl Into<String>, args: Vec<String>) -> Self {
    Self {
      command: command.into(),
      args,
      config: ConsumerConfig::default(),
    }
  }

  /// Set the configuration for this consumer
  pub fn with_config(mut self, config: ConsumerConfig) -> Self {
    self.config = config;
    self
  }
}

impl<T: AsRef<[u8]> + Send + 'static> EffectStreamSink<T, CommandStreamError> for CommandConsumer {
  type Stream = Pin<
    Box<
      dyn Future<Output = EffectResult<EffectStream<T, CommandStreamError>, CommandStreamError>>
        + Send
        + 'static,
    >,
  >;

  fn sink(&self) -> Self::Stream {
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

      let mut child = cmd
        .stdin(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| CommandStreamError::CommandStart(e.to_string()))?;

      let stdin = child.stdin.take().unwrap();
      let mut writer = BufWriter::new(stdin);
      let mut stream = EffectStream::<T, CommandStreamError>::new();

      tokio::spawn(async move {
        while let Ok(Some(value)) = stream.next().await {
          if let Err(e) = writer.write_all(value.as_ref()).await {
            stream.push_error(CommandStreamError::Io(e)).await.unwrap();
            break;
          }

          if config.append_newlines {
            if let Err(e) = writer.write_all(b"\n").await {
              stream.push_error(CommandStreamError::Io(e)).await.unwrap();
              break;
            }
          }
        }

        if let Err(e) = writer.flush().await {
          stream.push_error(CommandStreamError::Io(e)).await.unwrap();
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
  fn test_command_consumer() {
    let consumer = CommandConsumer::new("cat", vec![]);
    let stream = block_on(consumer.sink()).unwrap();

    block_on(async {
      stream.push("test".as_bytes()).await.unwrap();
      stream.close().await.unwrap();
    });
  }

  #[test]
  fn test_command_consumer_multiple_lines() {
    let consumer = CommandConsumer::new("cat", vec![]);
    let stream = block_on(consumer.sink()).unwrap();

    block_on(async {
      stream.push("line 1".as_bytes()).await.unwrap();
      stream.push("line 2".as_bytes()).await.unwrap();
      stream.push("line 3".as_bytes()).await.unwrap();
      stream.close().await.unwrap();
    });
  }

  #[test]
  fn test_command_consumer_nonexistent() {
    let consumer = CommandConsumer::new("nonexistent_command", vec![]);
    let result = block_on(consumer.sink());

    assert!(matches!(result, Err(CommandStreamError::CommandStart(_))));
  }
}

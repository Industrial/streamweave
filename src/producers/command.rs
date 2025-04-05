use crate::traits::{error::Error, output::Output, producer::Producer};
use futures::{Stream, StreamExt};
use std::error::Error as StdError;
use std::fmt;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio_stream::wrappers::LinesStream;

#[derive(Debug)]
pub enum CommandError {
  SpawnError(std::io::Error),
  StreamError(String),
}

impl fmt::Display for CommandError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      CommandError::SpawnError(e) => write!(f, "Failed to spawn command: {}", e),
      CommandError::StreamError(msg) => write!(f, "Stream error: {}", msg),
    }
  }
}

impl StdError for CommandError {
  fn source(&self) -> Option<&(dyn StdError + 'static)> {
    match self {
      CommandError::SpawnError(e) => Some(e),
      CommandError::StreamError(_) => None,
    }
  }
}

pub struct CommandProducer {
  command: String,
  args: Vec<String>,
}

impl CommandProducer {
  pub fn new(command: impl Into<String>, args: Vec<impl Into<String>>) -> Self {
    Self {
      command: command.into(),
      args: args.into_iter().map(|arg| arg.into()).collect(),
    }
  }
}

impl Error for CommandProducer {
  type Error = CommandError;
}

impl Output for CommandProducer {
  type Output = String;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, CommandError>> + Send>>;
}

impl Producer for CommandProducer {
  fn produce(&mut self) -> Self::OutputStream {
    let command_str = self.command.clone();
    let args = self.args.clone();

    // Create a stream that first spawns the command and then yields its output
    let stream = async_stream::try_stream! {
        let child = Command::new(&command_str)
            .args(&args)
            .stdout(Stdio::piped())
            .spawn()
            .map_err(CommandError::SpawnError)?;

        let stdout = child.stdout
            .ok_or_else(|| CommandError::StreamError("Failed to get stdout".to_string()))?;

        let reader = BufReader::new(stdout);
        let mut lines = reader.lines();

        while let Some(line) = lines.next_line()
            .await
            .map_err(|e| CommandError::StreamError(e.to_string()))?
        {
            yield line;
        }
    };

    Box::pin(stream)
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::StreamExt;

  #[tokio::test]
  async fn test_command_producer_echo() {
    let mut producer = CommandProducer::new("echo", vec!["Hello, World!"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["Hello, World!"]);
  }

  #[tokio::test]
  #[cfg(unix)] // This test is Unix-specific
  async fn test_command_producer_multiple_lines() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo 'line1\nline2\nline3'"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["line1", "line2", "line3"]);
  }

  #[tokio::test]
  async fn test_command_producer_nonexistent() {
    let mut producer = CommandProducer::new("nonexistentcommand", Vec::<String>::new());
    let stream = producer.produce();
    let result = stream.collect::<Vec<_>>().await;
    assert!(matches!(result[0], Err(CommandError::SpawnError(_))));
  }

  #[tokio::test]
  async fn test_multiple_calls() {
    let mut producer = CommandProducer::new("echo", vec!["test"]);

    // First call
    let stream = producer.produce();
    let result1: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result1, vec!["test"]);

    // Second call
    let stream = producer.produce();
    let result2: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result2, vec!["test"]);
  }

  #[tokio::test]
  #[cfg(unix)]
  async fn test_command_error_handling() {
    let mut producer = CommandProducer::new("sh", vec!["-c", "echo test && false"]);
    let stream = producer.produce();
    let result: Vec<String> = stream.map(|r| r.unwrap()).collect().await;
    assert_eq!(result, vec!["test"]);
  }
}

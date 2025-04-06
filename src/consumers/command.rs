use crate::traits::{consumer::Consumer, error::Error, input::Input};
use async_trait::async_trait;
use futures::StreamExt;
use std::ffi::OsStr;
use std::pin::Pin;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[derive(Debug)]
pub struct CommandConsumer {
  command: Command,
  delimiter: Option<Vec<u8>>,
}

#[derive(Debug)]
pub enum CommandConsumerError {
  IoError(std::io::Error),
  ProcessError(i32),
}

impl std::fmt::Display for CommandConsumerError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      CommandConsumerError::IoError(e) => write!(f, "Command I/O error: {}", e),
      CommandConsumerError::ProcessError(code) => {
        write!(f, "Command failed with exit code: {}", code)
      }
    }
  }
}

impl std::error::Error for CommandConsumerError {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    match self {
      CommandConsumerError::IoError(e) => Some(e),
      CommandConsumerError::ProcessError(_) => None,
    }
  }
}

impl From<std::io::Error> for CommandConsumerError {
  fn from(error: std::io::Error) -> Self {
    CommandConsumerError::IoError(error)
  }
}

impl Error for CommandConsumer {
  type Error = CommandConsumerError;
}

impl Input for CommandConsumer {
  type Input = Vec<u8>;
  type InputStream = Pin<Box<dyn futures::Stream<Item = Result<Self::Input, Self::Error>> + Send>>;
}

impl CommandConsumer {
  pub fn new(program: impl AsRef<OsStr>) -> Self {
    let mut cmd = Command::new(program.as_ref());
    cmd.stdin(Stdio::piped());
    Self {
      command: cmd,
      delimiter: None,
    }
  }

  pub fn with_delimiter(mut self, delimiter: Vec<u8>) -> Self {
    self.delimiter = Some(delimiter);
    self
  }

  pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
    self.command.arg(arg);
    self
  }

  pub fn args<I, S>(mut self, args: I) -> Self
  where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
  {
    self.command.args(args);
    self
  }
}

#[async_trait]
impl Consumer for CommandConsumer {
  async fn consume(&mut self, mut input: Self::InputStream) -> Result<(), Self::Error> {
    let mut child = self.command.spawn()?;
    let stdin = child
      .stdin
      .as_mut()
      .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Failed to open stdin"))?;

    while let Some(result) = input.next().await {
      let data = result?;
      stdin.write_all(&data).await?;
      if let Some(delimiter) = &self.delimiter {
        stdin.write_all(delimiter).await?;
      }
    }

    drop(stdin); // Close stdin to signal EOF
    let status = child.wait().await?;

    if !status.success() {
      return Err(CommandConsumerError::ProcessError(
        status.code().unwrap_or(-1),
      ));
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use futures::stream;
  use std::path::Path;
  use tokio;

  #[tokio::test]
  async fn test_empty_stream() {
    let mut consumer = CommandConsumer::new("echo");
    let input = stream::empty();
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_basic_command() {
    let mut consumer = CommandConsumer::new("echo");
    let input = stream::iter(vec![Ok(b"hello world".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_with_args() {
    let mut consumer = CommandConsumer::new("echo").arg("-n").arg("test");
    let input = stream::iter(vec![Ok(b"hello".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_with_multiple_args() {
    let mut consumer = CommandConsumer::new("echo").args(vec!["-n", "test"]);
    let input = stream::iter(vec![Ok(b"hello".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_with_delimiter() {
    let mut consumer = CommandConsumer::new("echo").with_delimiter(b"\n".to_vec());
    let input = stream::iter(vec![Ok(b"hello".to_vec()), Ok(b"world".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_with_path() {
    let mut consumer = CommandConsumer::new(Path::new("echo"));
    let input = stream::iter(vec![Ok(b"hello".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_command_with_error() {
    let mut consumer = if cfg!(windows) {
      CommandConsumer::new("cmd").arg("/C").arg("exit /b 1")
    } else {
      CommandConsumer::new("sh").arg("-c").arg("exit 1")
    };
    let input = stream::iter(vec![Ok(b"hello".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      CommandConsumerError::ProcessError(_)
    ));
  }

  #[tokio::test]
  async fn test_nonexistent_command() {
    let mut consumer = CommandConsumer::new("nonexistent_command_123456");
    let input = stream::iter(vec![Ok(b"hello".to_vec())]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      CommandConsumerError::IoError(_)
    ));
  }

  #[tokio::test]
  async fn test_large_input() {
    let mut consumer = CommandConsumer::new("cat");
    let data = vec![b'a'; 1024 * 1024]; // 1MB of data
    let input = stream::iter(vec![Ok(data)]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_multiple_chunks() {
    let mut consumer = CommandConsumer::new("cat");
    let input = stream::iter(vec![
      Ok(b"hello".to_vec()),
      Ok(b" ".to_vec()),
      Ok(b"world".to_vec()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }

  #[tokio::test]
  async fn test_error_propagation() {
    let mut consumer = CommandConsumer::new("cat");
    let input = stream::iter(vec![
      Ok(b"hello".to_vec()),
      Err(CommandConsumerError::IoError(std::io::Error::new(
        std::io::ErrorKind::Other,
        "test error",
      ))),
      Ok(b"world".to_vec()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_err());
    assert!(matches!(
      result.unwrap_err(),
      CommandConsumerError::IoError(_)
    ));
  }

  #[tokio::test]
  async fn test_command_with_special_chars() {
    let mut consumer = CommandConsumer::new("echo");
    let input = stream::iter(vec![
      Ok(b"hello\nworld".to_vec()),
      Ok(b"\x00\x01\x02".to_vec()),
    ]);
    let result = consumer.consume(Box::pin(input)).await;
    assert!(result.is_ok());
  }
}

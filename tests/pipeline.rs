use async_trait::async_trait;
use futures::{Stream, StreamExt, stream};
use std::error::Error;
use std::fmt;
use std::pin::Pin;
use streamweave::traits::{consumer::Consumer, producer::Producer, transformer::Transformer};

// Test types and errors
#[derive(Debug)]
enum TestError {
  Producer(String),
  Transformer(String),
  Consumer(String),
}

impl fmt::Display for TestError {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    match self {
      Self::Producer(msg) => write!(f, "Producer error: {}", msg),
      Self::Transformer(msg) => write!(f, "Transformer error: {}", msg),
      Self::Consumer(msg) => write!(f, "Consumer error: {}", msg),
    }
  }
}

impl Error for TestError {}

// A producer that generates numbers
struct NumberProducer {
  initialized: bool,
  range: std::ops::Range<i32>,
}

impl Producer for NumberProducer {
  type Output = i32;
  type Error = TestError;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;

  fn produce(&mut self) -> Self::OutputStream {
    if !self.initialized {
      Box::pin(stream::empty())
    } else {
      Box::pin(stream::iter(self.range.clone()).map(Ok))
    }
  }

  async fn init(&mut self) -> Result<(), Self::Error> {
    self.initialized = true;
    Ok(())
  }

  async fn shutdown(&mut self) -> Result<(), Self::Error> {
    self.initialized = false;
    Ok(())
  }
}

// A transformer that doubles numbers
struct DoubleTransformer;

impl Transformer for DoubleTransformer {
  type Input = i32;
  type Output = i32;
  type Error = TestError;
  type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|r| r.map(|x| x * 2)))
  }

  async fn transform_async(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error> {
    Ok(input * 2)
  }
}

// A transformer that converts numbers to strings
struct StringifyTransformer;

impl Transformer for StringifyTransformer {
  type Input = i32;
  type Output = String;
  type Error = TestError;
  type InputStream = Pin<Box<dyn Stream<Item = Result<i32, TestError>> + Send>>;
  type OutputStream = Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>>;

  fn transform(&mut self, input: Self::InputStream) -> Self::OutputStream {
    Box::pin(input.map(|r| r.map(|x| x.to_string())))
  }

  async fn transform_async(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error> {
    Ok(input.to_string())
  }
}

// A consumer that collects strings
struct CollectorConsumer {
  collected: Vec<String>,
}

impl Consumer for CollectorConsumer {
  type Input = String;
  type Error = TestError;
  type InputStream = Pin<Box<dyn Stream<Item = Result<String, TestError>> + Send>>;

  async fn consume(&mut self, mut stream: Self::InputStream) -> Result<(), Self::Error> {
    while let Some(result) = stream.next().await {
      match result {
        Ok(item) => self.collected.push(item),
        Err(e) => return Err(e),
      }
    }
    Ok(())
  }
}

#[tokio::test]
async fn test_pipeline() {
  let producer = NumberProducer {
    initialized: true,
    range: 0..3,
  };
  let transformers = vec![DoubleTransformer, StringifyTransformer];
  let consumer = CollectorConsumer {
    collected: Vec::new(),
  };

  let pipeline = Pipeline::new(producer, transformers, consumer);
  pipeline.run().await.unwrap();
}

#[tokio::test]
async fn test_pipeline_error_propagation() {
  // A producer that fails after generating some items
  struct FailingProducer {
    initialized: bool,
    fail_after: usize,
    current: usize,
  }

  #[async_trait]
  impl Producer for FailingProducer {
    type Output = i32;
    type Error = TestError;

    fn produce(&mut self) -> Pin<Box<dyn Stream<Item = Self::Output> + Send + '_>> {
      if !self.initialized {
        return Box::pin(stream::iter(vec![]));
      }

      // Only generate items up to fail_after
      let items: Vec<i32> = (0..self.fail_after).map(|i| i as i32).collect();

      Box::pin(stream::iter(items))
    }

    async fn init(&mut self) -> Result<(), Self::Error> {
      if self.initialized {
        return Err(TestError::Producer("Already initialized".to_string()));
      }
      self.initialized = true;
      Ok(())
    }

    async fn shutdown(&mut self) -> Result<(), Self::Error> {
      self.initialized = false;
      Ok(())
    }
  }

  // A transformer that fails on certain values
  struct FailingTransformer {
    fail_on: i32,
  }

  #[async_trait]
  impl Transformer for FailingTransformer {
    type Input = i32;
    type Output = i32;
    type Error = TestError;

    fn transform(
      &mut self,
      input: Pin<Box<dyn Stream<Item = Self::Input> + Send>>,
    ) -> Pin<Box<dyn Stream<Item = Self::Output> + Send>> {
      let fail_on = self.fail_on;
      Box::pin(input.map(move |x| {
        if x == fail_on {
          panic!("Transformer failed on value {}", x);
        }
        x * 2
      }))
    }

    async fn transform_async(&mut self, input: Self::Input) -> Result<Self::Output, Self::Error> {
      if input == self.fail_on {
        Err(TestError::Transformer(format!("Failed on value {}", input)))
      } else {
        Ok(input * 2)
      }
    }
  }

  // A consumer that fails after consuming some items
  struct FailingConsumer {
    collected: Vec<i32>,
    fail_after: usize,
  }

  #[async_trait]
  impl Consumer for FailingConsumer {
    type Input = i32;
    type Error = TestError;

    async fn consume(
      &mut self,
      mut stream: Pin<Box<dyn Stream<Item = Self::Input> + Send>>,
    ) -> Result<(), Self::Error> {
      while let Some(item) = stream.next().await {
        if self.collected.len() >= self.fail_after {
          return Err(TestError::Consumer(format!(
            "Failed after {} items",
            self.fail_after
          )));
        }
        self.collected.push(item);
      }
      Ok(())
    }
  }

  // Test 1: Producer failure
  {
    let mut producer = FailingProducer {
      initialized: false,
      fail_after: 3,
      current: 0,
    };
    producer
      .init()
      .await
      .expect("Failed to initialize producer");

    let mut stream = producer.produce();
    let mut collected = Vec::new();

    while let Some(item) = stream.next().await {
      collected.push(item);
    }

    assert_eq!(collected.len(), 3, "Should collect exactly 3 items");
    assert_eq!(collected, vec![0, 1, 2], "Should collect first 3 numbers");
  }

  // Test 2: Transformer failure
  {
    let mut producer = NumberProducer {
      initialized: false,
      range: 0..5,
    };
    let mut transformer = FailingTransformer { fail_on: 3 };
    producer
      .init()
      .await
      .expect("Failed to initialize producer");

    let numbers: Vec<i32> = producer.produce().collect().await;
    let result = transformer.transform_async(3).await;
    assert!(result.is_err(), "Transformer should fail on value 3");
    assert!(matches!(
        result.unwrap_err(),
        TestError::Transformer(msg) if msg.contains("3")
    ));
  }

  // Test 3: Consumer failure
  {
    let mut producer = NumberProducer {
      initialized: false,
      range: 0..5,
    };
    let mut consumer = FailingConsumer {
      collected: Vec::new(),
      fail_after: 2,
    };
    producer
      .init()
      .await
      .expect("Failed to initialize producer");

    let numbers: Vec<i32> = producer.produce().collect().await;
    let result = consumer.consume(Box::pin(stream::iter(numbers))).await;
    assert!(result.is_err(), "Consumer should fail after 2 items");
    assert!(matches!(
        result.unwrap_err(),
        TestError::Consumer(msg) if msg.contains("2")
    ));
    assert_eq!(
      consumer.collected.len(),
      2,
      "Should collect 2 items before failing"
    );
  }
}

#[tokio::test]
async fn test_pipeline_backpressure() {
  // Test how backpressure works through the pipeline
  // This would test streaming behavior between components
  // ...
}

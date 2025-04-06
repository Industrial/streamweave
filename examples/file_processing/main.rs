use std::error::Error;
use std::path::Path;
use streamweave::{
  consumers::string::StringConsumer, pipeline::PipelineBuilder, producers::file::FileProducer,
  transformers::map::MapTransformer,
};
use tempfile::tempdir;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
  // Create a temporary directory for our test files
  let temp_dir = tempdir()?;
  let input_path = temp_dir.path().join("input.txt");

  // Create input file with some text
  tokio::fs::write(
    &input_path,
    "Hello\nWorld\n123\n456\n789\nStreamWeave\nExample\n",
  )
  .await?;

  // Create a pipeline that:
  // 1. Reads from input file (produces strings)
  // 2. Transforms each line to uppercase
  // 3. Collects all strings into one with newlines
  let pipeline = PipelineBuilder::new()
    .producer(FileProducer::new(&input_path))
    .transformer(MapTransformer::new(|line: String| line.to_uppercase()))
    .consumer(StringConsumer::with_separator("\n"));

  // Run the pipeline
  let (_, consumer) = pipeline.run().await?;

  // Print the result
  println!("File contents:\n{}", consumer.into_string());

  Ok(())
}

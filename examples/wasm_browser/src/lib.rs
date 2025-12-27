//! StreamWeave WASM module for browser usage
//!
//! This is a demonstration example showing how to structure a WASM module
//! that uses StreamWeave. In a real implementation, you would use async
//! execution with wasm-bindgen-futures to run StreamWeave pipelines.
//!
//! Note: This is a simplified example for documentation purposes.
//! Full async pipeline execution requires proper async runtime setup.

use wasm_bindgen::prelude::*;

/// Example: Process a simple array of numbers
///
/// This demonstrates the basic structure for using StreamWeave in WASM.
/// In production, you would use async pipelines with wasm-bindgen-futures.
#[wasm_bindgen]
pub fn process_numbers(numbers: &[u32]) -> Vec<u32> {
  // This is a simplified synchronous example
  // Real StreamWeave usage would be async:
  //
  // use wasm_bindgen_futures::JsFuture;
  // use streamweave::pipeline::Pipeline;
  // use streamweave::producers::array::ArrayProducer;
  // use streamweave::transformers::map::MapTransformer;
  // use streamweave::consumers::vec::VecConsumer;
  //
  // #[wasm_bindgen]
  // pub async fn process_async(numbers: Vec<u32>) -> Vec<u32> {
  //     let pipeline = Pipeline::<u32, u32>::new()
  //         .producer(ArrayProducer::new(numbers))
  //         .transformer(MapTransformer::new(|x| x * 2))
  //         .consumer(VecConsumer::new());
  //
  //     pipeline.run().await.unwrap()
  // }

  numbers.iter().map(|x| x * 2).collect()
}

/// Example: Process a string
#[wasm_bindgen]
pub fn process_string(input: &str) -> String {
  input.to_uppercase()
}

/// Example: Calculate sum of numbers
#[wasm_bindgen]
pub fn calculate_sum(numbers: &[u32]) -> u32 {
  numbers.iter().sum()
}

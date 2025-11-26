//! StreamWeave WASM module for Node.js usage
//!
//! This is a demonstration example showing how to structure a WASM module
//! that uses StreamWeave in Node.js. In a real implementation, you would
//! use async execution with wasm-bindgen-futures.
//!
//! Note: This is a simplified example for documentation purposes.
//! Full async pipeline execution requires proper async runtime setup.

use wasm_bindgen::prelude::*;

/// Example: Process a stream of numbers
///
/// This demonstrates the basic structure for using StreamWeave in Node.js.
/// In production, you would use async pipelines with wasm-bindgen-futures.
#[wasm_bindgen]
pub fn process_stream(numbers: &[u32]) -> Vec<u32> {
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
    //         .with_producer(ArrayProducer::new(numbers))
    //         .with_transformer(MapTransformer::new(|x| x * 2))
    //         .with_consumer(VecConsumer::new());
    //     
    //     pipeline.run().await.unwrap()
    // }
    
    numbers.iter().map(|x| x * 2).filter(|&x| x > 10).collect()
}

/// Example: Calculate statistics on an array of numbers
#[wasm_bindgen]
pub fn calculate_stats(numbers: &[u32]) -> String {
    if numbers.is_empty() {
        return r#"{"count": 0, "sum": 0, "avg": 0}"#.to_string();
    }
    
    let count = numbers.len();
    let sum: u32 = numbers.iter().sum();
    let avg = sum as f64 / count as f64;
    
    format!(
        r#"{{"count": {}, "sum": {}, "avg": {:.2}}}"#,
        count, sum, avg
    )
}


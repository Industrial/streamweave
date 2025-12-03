//! Pipeline setup for visualization_html example
//!
//! This module creates a complex multi-stage pipeline with:
//! - Producer: ArrayProducer
//! - Transformer 1: MapTransformer (transforms data)
//! - Transformer 2: FilterTransformer (filters data)
//! - Consumer: VecConsumer

use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, producers::array::array_producer::ArrayProducer,
  transformers::filter::filter_transformer::FilterTransformer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates a complex multi-stage pipeline for visualization demonstration.
///
/// The pipeline consists of:
/// 1. Producer: Generates numbers from 1 to 20
/// 2. Transformer 1: Squares each number
/// 3. Transformer 2: Filters to keep only numbers greater than 50
/// 4. Consumer: Collects results into a vector
///
/// # Returns
///
/// A tuple containing the producer, two transformers, and consumer components
/// that can be used to create a pipeline and generate a DAG.
#[allow(clippy::type_complexity)]
pub fn create_complex_pipeline() -> (
  ArrayProducer<i32, 20>,
  MapTransformer<impl FnMut(i32) -> i32 + Send + Clone + 'static, i32, i32>,
  FilterTransformer<impl FnMut(&i32) -> bool + Send + Clone + 'static, i32>,
  VecConsumer<i32>,
) {
  let producer = ArrayProducer::new([
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20,
  ])
  .with_name("numbers_producer".to_string());

  let transformer1 = MapTransformer::new(|x: i32| x * x).with_name("squarer".to_string());

  let transformer2 =
    FilterTransformer::new(|x: &i32| *x > 50).with_name("greater_than_50_filter".to_string());

  let consumer = VecConsumer::new().with_name("results_collector".to_string());

  (producer, transformer1, transformer2, consumer)
}

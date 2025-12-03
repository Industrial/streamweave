//! Pipeline setup for visualization_basic example
//!
//! This module creates a multi-stage pipeline with:
//! - Producer: ArrayProducer
//! - Transformer 1: MapTransformer (doubles values)
//! - Transformer 2: FilterTransformer (filters even numbers)
//! - Consumer: VecConsumer

use streamweave::{
  consumers::vec::vec_consumer::VecConsumer, producers::array::array_producer::ArrayProducer,
  transformers::filter::filter_transformer::FilterTransformer,
  transformers::map::map_transformer::MapTransformer,
};

/// Creates a multi-stage pipeline for visualization demonstration.
///
/// The pipeline consists of:
/// 1. Producer: Generates numbers from 1 to 10
/// 2. Transformer 1: Doubles each number
/// 3. Transformer 2: Filters to keep only even numbers (which will be all after doubling)
/// 4. Consumer: Collects results into a vector
///
/// # Returns
///
/// A tuple containing the producer, two transformers, and consumer components
/// that can be used to create a pipeline and generate a DAG.
#[allow(clippy::type_complexity)]
pub fn create_multi_stage_pipeline() -> (
  ArrayProducer<i32, 10>,
  MapTransformer<impl FnMut(i32) -> i32 + Send + Clone + 'static, i32, i32>,
  FilterTransformer<impl FnMut(&i32) -> bool + Send + Clone + 'static, i32>,
  VecConsumer<i32>,
) {
  let producer =
    ArrayProducer::new([1, 2, 3, 4, 5, 6, 7, 8, 9, 10]).with_name("numbers_producer".to_string());

  let transformer1 = MapTransformer::new(|x: i32| x * 2).with_name("doubler".to_string());

  let transformer2 =
    FilterTransformer::new(|x: &i32| x % 2 == 0).with_name("even_filter".to_string());

  let consumer = VecConsumer::new().with_name("results_collector".to_string());

  (producer, transformer1, transformer2, consumer)
}

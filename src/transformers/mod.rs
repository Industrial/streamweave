//! # Transformers Module
//!
//! Comprehensive collection of transformer implementations for the StreamWeave framework.
//! Transformers are components that process data streams, transforming input items into
//! output items as they flow through pipelines and graphs.
//!
//! ## Overview
//!
//! This module provides over 100 transformer implementations organized by category:
//!
//! - **Array Operations**: Array manipulation, searching, and transformation
//! - **String Operations**: String manipulation, parsing, and formatting
//! - **JSON Operations**: JSON parsing, stringification, reading, and writing
//! - **CSV Operations**: CSV parsing, reading, writing, and stringification
//! - **Database Operations**: Database queries and writes for PostgreSQL, MySQL, SQLite
//! - **File System Operations**: File and directory operations, path manipulation
//! - **Network Operations**: HTTP requests, TCP operations, Kafka publishing
//! - **Mathematical Operations**: Math functions, operations, and utilities
//! - **Object Operations**: Object manipulation, property access, merging
//! - **Stream Operations**: Batching, filtering, mapping, merging, splitting
//! - **Control Flow**: Circuit breakers, retries, rate limiting, delays
//! - **Machine Learning**: ML inference and batched inference transformers
//! - **Data Formats**: Parquet, JSONL, XML parsing and writing
//!
//! ## Universal Message Model
//!
//! **All transformers operate on `Message<T>` where `T` is the payload type.**
//! This ensures message IDs, metadata, and error correlation are preserved throughout
//! the transformation pipeline.
//!
//! ## Error Handling
//!
//! All transformers support configurable error strategies:
//!
//! - **Stop**: Stop processing on error
//! - **Skip**: Skip items that cause errors
//! - **Retry**: Retry failed items with configurable retry count
//! - **Custom**: Custom error handling logic
//!
//! ## Configuration
//!
//! Transformers can be configured with:
//!
//! - Error handling strategies
//! - Component names for debugging
//! - Transformer-specific options
//!
//! ## Example Usage
//!
//! ```rust
//! use crate::transformers::MapTransformer;
//! use crate::message::Message;
//!
//! let transformer = MapTransformer::new(|x: i32| x * 2);
//! // Transforms [1, 2, 3] into [2, 4, 6]
//! ```
pub mod array_concat_transformer;
pub mod array_contains_transformer;
pub mod array_find_transformer;
pub mod array_index_of_transformer;
pub mod array_join_transformer;
pub mod array_length_transformer;
pub mod array_modify_transformer;
pub mod array_reverse_transformer;
pub mod array_slice_transformer;
pub mod batch_transformer;
pub mod circuit_breaker_transformer;
pub mod command_execute_transformer;
pub mod csv_parse_transformer;
pub mod csv_read_transformer;
pub mod csv_stringify_transformer;
pub mod csv_write_transformer;
pub mod database_operation_transformer;
pub mod database_query_transformer;
pub mod db_mysql_query_transformer;
pub mod db_mysql_write_transformer;
pub mod db_postgres_query_transformer;
pub mod db_postgres_write_transformer;
pub mod db_sqlite_query_transformer;
pub mod db_sqlite_write_transformer;
pub mod delay_transformer;
pub mod drop_transformer;
pub mod filter_transformer;
pub mod fs_directory_create_transformer;
pub mod fs_directory_list_transformer;
pub mod fs_file_name_transformer;
pub mod fs_file_read_transformer;
pub mod fs_file_write_transformer;
pub mod fs_normalize_path_transformer;
pub mod fs_parent_path_transformer;
pub mod group_by_transformer;
pub mod http_request_transformer;
pub mod interleave_transformer;
pub mod json_parse_transformer;
pub mod json_read_transformer;
pub mod json_stringify_transformer;
pub mod json_write_transformer;
pub mod jsonl_read_transformer;
pub mod jsonl_write_transformer;
pub mod jsonpath_transformer;
pub mod kafka_publish_transformer;
pub mod limit_transformer;
pub mod map_transformer;
pub mod math_function_transformer;
pub mod math_hyperbolic_transformer;
pub mod math_logarithmic_transformer;
pub mod math_min_max_transformer;
pub mod math_operation_transformer;
pub mod math_random_transformer;
pub mod math_rounding_transformer;
pub mod math_trigonometric_transformer;
pub mod math_utility_transformer;
pub mod merge_transformer;
pub mod message_dedupe_transformer;
pub mod ml_batched_inference_transformer;
pub mod ml_inference_transformer;
pub mod object_entries_transformer;
pub mod object_has_property_transformer;
pub mod object_keys_transformer;
pub mod object_merge_transformer;
pub mod object_property_transformer;
pub mod object_random_member_transformer;
pub mod object_values_transformer;
pub mod ordered_merge_transformer;
pub mod parquet_read_transformer;
pub mod parquet_write_transformer;
pub mod partition_transformer;
pub mod process_execute_transformer;
pub mod process_transformer;
pub mod rate_limit_transformer;
pub mod redis_write_transformer;
pub mod reduce_transformer;
pub mod repeat_transformer;
pub mod retry_transformer;
pub mod round_robin_transformer;
pub mod router_transformer;
pub mod sample_transformer;
pub mod skip_transformer;
pub mod sort_transformer;
pub mod split_at_transformer;
pub mod split_transformer;
pub mod string_case_transformer;
pub mod string_char_transformer;
pub mod string_concat_transformer;
pub mod string_index_of_transformer;
pub mod string_join_transformer;
pub mod string_length_transformer;
pub mod string_match_transformer;
pub mod string_pad_transformer;
pub mod string_predicate_transformer;
pub mod string_repeat_transformer;
pub mod string_replace_transformer;
pub mod string_reverse_transformer;
pub mod string_search_transformer;
pub mod string_slice_transformer;
pub mod string_split_lines_transformer;
pub mod string_split_transformer;
pub mod string_split_words_transformer;
pub mod string_trim_transformer;
pub mod take_transformer;
pub mod tcp_receive_transformer;
pub mod tcp_request_transformer;
pub mod tcp_send_transformer;
pub mod time_window_transformer;
pub mod timeout_transformer;
pub mod tokio_channel_send_transformer;
pub mod trace_transformer;
pub mod window_transformer;
pub mod wrap_transformer;
pub mod xml_parse_transformer;
pub mod xml_stringify_transformer;
pub mod xor_transformer;
pub mod zip_transformer;

pub use array_concat_transformer::*;
pub use array_contains_transformer::*;
pub use array_find_transformer::*;
pub use array_index_of_transformer::*;
pub use array_join_transformer::*;
pub use array_length_transformer::*;
pub use array_modify_transformer::*;
pub use array_reverse_transformer::*;
pub use array_slice_transformer::*;
pub use batch_transformer::*;
pub use circuit_breaker_transformer::*;
pub use command_execute_transformer::*;
pub use csv_parse_transformer::*;
pub use csv_read_transformer::*;
pub use csv_stringify_transformer::*;
pub use csv_write_transformer::*;
pub use database_operation_transformer::*;
pub use database_query_transformer::*;
pub use db_mysql_query_transformer::*;
pub use db_mysql_write_transformer::*;
pub use db_postgres_query_transformer::*;
pub use db_postgres_write_transformer::*;
pub use db_sqlite_query_transformer::*;
pub use db_sqlite_write_transformer::*;
pub use delay_transformer::*;
pub use drop_transformer::*;
pub use filter_transformer::*;
pub use fs_directory_create_transformer::*;
pub use fs_directory_list_transformer::*;
pub use fs_file_name_transformer::*;
pub use fs_file_read_transformer::*;
pub use fs_file_write_transformer::*;
pub use fs_normalize_path_transformer::*;
pub use fs_parent_path_transformer::*;
pub use group_by_transformer::*;
pub use http_request_transformer::*;
pub use interleave_transformer::*;
pub use json_parse_transformer::*;
pub use json_read_transformer::*;
pub use json_stringify_transformer::*;
pub use json_write_transformer::*;
pub use jsonl_read_transformer::*;
pub use jsonl_write_transformer::*;
pub use jsonpath_transformer::*;
pub use kafka_publish_transformer::*;
pub use limit_transformer::*;
pub use map_transformer::*;
pub use math_function_transformer::*;
pub use math_hyperbolic_transformer::*;
pub use math_logarithmic_transformer::*;
pub use math_min_max_transformer::*;
pub use math_operation_transformer::*;
pub use math_random_transformer::*;
pub use math_rounding_transformer::*;
pub use math_trigonometric_transformer::*;
pub use math_utility_transformer::*;
pub use merge_transformer::*;
pub use message_dedupe_transformer::*;
pub use ml_batched_inference_transformer::*;
pub use ml_inference_transformer::*;
pub use object_entries_transformer::*;
pub use object_has_property_transformer::*;
pub use object_keys_transformer::*;
pub use object_merge_transformer::*;
pub use object_property_transformer::*;
pub use object_random_member_transformer::*;
pub use object_values_transformer::*;
pub use ordered_merge_transformer::*;
pub use parquet_read_transformer::*;
pub use parquet_write_transformer::*;
pub use partition_transformer::*;
pub use process_execute_transformer::*;
pub use process_transformer::*;
pub use rate_limit_transformer::*;
pub use redis_write_transformer::*;
pub use reduce_transformer::*;
pub use repeat_transformer::*;
pub use retry_transformer::*;
pub use round_robin_transformer::*;
pub use router_transformer::*;
pub use sample_transformer::*;
pub use skip_transformer::*;
pub use sort_transformer::*;
pub use split_at_transformer::*;
pub use split_transformer::*;
pub use string_case_transformer::*;
pub use string_char_transformer::*;
pub use string_concat_transformer::*;
pub use string_index_of_transformer::*;
pub use string_join_transformer::*;
pub use string_length_transformer::*;
pub use string_match_transformer::*;
pub use string_pad_transformer::*;
pub use string_predicate_transformer::*;
pub use string_repeat_transformer::*;
pub use string_replace_transformer::*;
pub use string_reverse_transformer::*;
pub use string_search_transformer::*;
pub use string_slice_transformer::*;
pub use string_split_lines_transformer::*;
pub use string_split_transformer::*;
pub use string_split_words_transformer::*;
pub use string_trim_transformer::*;
pub use take_transformer::*;
pub use tcp_receive_transformer::*;
pub use tcp_request_transformer::*;
pub use tcp_send_transformer::*;
pub use time_window_transformer::*;
pub use timeout_transformer::*;
pub use tokio_channel_send_transformer::*;
pub use trace_transformer::*;
pub use window_transformer::*;
pub use wrap_transformer::*;
pub use xml_parse_transformer::*;
pub use xml_stringify_transformer::*;
pub use xor_transformer::*;
pub use zip_transformer::*;

#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use serde::{Deserialize, Serialize};
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use streamweave::{
  consumers::csv::csv_consumer::CsvConsumer, consumers::jsonl::jsonl_consumer::JsonlConsumer,
  consumers::parquet::parquet_consumer::ParquetConsumer, consumers::vec::vec_consumer::VecConsumer,
  error::ErrorStrategy, pipeline::PipelineBuilder, producers::csv::csv_producer::CsvProducer,
  producers::jsonl::jsonl_producer::JsonlProducer,
  transformers::map::map_transformer::MapTransformer,
};

#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use arrow::array::{ArrayRef, Int64Array, StringArray};
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use arrow::datatypes::{DataType, Field, Schema};
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use arrow::record_batch::RecordBatch;
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
use std::sync::Arc;

/// Example data structure for CSV and JSONL
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
#[derive(Debug, Clone, Serialize, Deserialize)]
struct Person {
  id: u32,
  name: String,
  email: String,
  age: u32,
  active: bool,
}

/// Example: CSV read/write with headers
///
/// This demonstrates:
/// - Reading CSV files with header rows
/// - Streaming CSV parsing (doesn't load entire file)
/// - Writing CSV files with headers
/// - Handling different data types in CSV
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub async fn csv_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📄 Setting up CSV read/write example...");

  // Read CSV file with headers
  let producer = CsvProducer::<Person>::new("examples/file_formats/data/sample.csv")
    .with_name("csv-reader".to_string())
    .with_headers(true) // File has headers
    .with_error_strategy(ErrorStrategy::Skip); // Skip malformed rows

  // Transform to format for display
  let transformer = MapTransformer::new(|person: Person| -> String {
    format!(
      "Person #{}: {} ({}) - Age: {}, Active: {}",
      person.id, person.name, person.email, person.age, person.active
    )
  })
  .with_error_strategy(ErrorStrategy::Skip);

  // Collect results
  let consumer = VecConsumer::new();

  println!("🔄 Reading CSV file and processing...");
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("\n✅ CSV read completed!");
  println!("📊 Results ({} records):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  // Now write CSV file
  println!("\n📝 Writing CSV file...");
  let write_producer = streamweave::producers::vec::vec_producer::VecProducer::new(results.clone());
  let write_transformer = MapTransformer::new(|s: String| -> Person {
    // Parse back from display format (simplified - in real use, keep original data)
    // For demo, we'll create new data
    Person {
      id: 1,
      name: "Test".to_string(),
      email: "test@example.com".to_string(),
      age: 25,
      active: true,
    }
  });

  // Create sample data for writing
  let sample_data = vec![
    Person {
      id: 10,
      name: "Frank".to_string(),
      email: "frank@example.com".to_string(),
      age: 40,
      active: true,
    },
    Person {
      id: 11,
      name: "Grace".to_string(),
      email: "grace@example.com".to_string(),
      age: 27,
      active: false,
    },
  ];

  let write_producer = streamweave::producers::vec::vec_producer::VecProducer::new(sample_data);
  let write_consumer = CsvConsumer::<Person>::new("examples/file_formats/data/output.csv")
    .with_name("csv-writer".to_string())
    .with_headers(true) // Write headers
    .with_error_strategy(ErrorStrategy::Skip);

  let write_pipeline = PipelineBuilder::new()
    .with_producer(write_producer)
    .with_consumer(write_consumer)
    .build();

  write_pipeline.run().await?;

  println!("✅ CSV write completed! Check examples/file_formats/data/output.csv");

  Ok(())
}

/// Example: JSONL streaming for large files
///
/// This demonstrates:
/// - Streaming JSONL parsing (line-by-line, doesn't load entire file)
/// - Handling large files efficiently
/// - Writing JSONL files
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub async fn jsonl_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📄 Setting up JSONL streaming example...");

  // Read JSONL file (streams line by line)
  let producer = JsonlProducer::<Person>::new("examples/file_formats/data/sample.jsonl")
    .with_name("jsonl-reader".to_string())
    .with_error_strategy(ErrorStrategy::Skip); // Skip malformed lines

  // Transform to format for display
  let transformer = MapTransformer::new(|person: Person| -> String {
    format!(
      "Person #{}: {} - Age: {}",
      person.id, person.name, person.age
    )
  })
  .with_error_strategy(ErrorStrategy::Skip);

  // Collect results
  let consumer = VecConsumer::new();

  println!("🔄 Streaming JSONL file (line-by-line processing)...");
  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("\n✅ JSONL read completed!");
  println!("📊 Results ({} records):", results.len());
  for (i, result) in results.iter().enumerate() {
    println!("  {}. {}", i + 1, result);
  }

  // Write JSONL file
  println!("\n📝 Writing JSONL file...");
  let sample_data = vec![
    Person {
      id: 20,
      name: "Henry".to_string(),
      email: "henry@example.com".to_string(),
      age: 33,
      active: true,
    },
    Person {
      id: 21,
      name: "Iris".to_string(),
      email: "iris@example.com".to_string(),
      age: 29,
      active: true,
    },
  ];

  let write_producer = streamweave::producers::vec::vec_producer::VecProducer::new(sample_data);
  let write_consumer = JsonlConsumer::<Person>::new("examples/file_formats/data/output.jsonl")
    .with_name("jsonl-writer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let write_pipeline = PipelineBuilder::new()
    .with_producer(write_producer)
    .with_consumer(write_consumer)
    .build();

  write_pipeline.run().await?;

  println!("✅ JSONL write completed! Check examples/file_formats/data/output.jsonl");

  Ok(())
}

/// Example: Parquet column projection
///
/// This demonstrates:
/// - Reading Parquet files with column projection (only selected columns)
/// - Efficient columnar reading
/// - Writing Parquet files
/// - Working with Arrow RecordBatches
#[cfg(all(not(target_arch = "wasm32"), feature = "file-formats"))]
pub async fn parquet_example() -> Result<(), Box<dyn std::error::Error>> {
  println!("📄 Setting up Parquet column projection example...");

  // First, create a sample Parquet file
  println!("📝 Creating sample Parquet file...");
  let schema = Arc::new(Schema::new(vec![
    Field::new("id", DataType::Int64, false),
    Field::new("name", DataType::Utf8, false),
    Field::new("email", DataType::Utf8, false),
    Field::new("age", DataType::Int64, false),
    Field::new("score", DataType::Int64, false), // Extra column for projection demo
  ]));

  let id_array = Arc::new(Int64Array::from(vec![1, 2, 3, 4, 5]));
  let name_array = Arc::new(StringArray::from(vec![
    "Alice", "Bob", "Charlie", "Diana", "Eve",
  ]));
  let email_array = Arc::new(StringArray::from(vec![
    "alice@example.com",
    "bob@example.com",
    "charlie@example.com",
    "diana@example.com",
    "eve@example.com",
  ]));
  let age_array = Arc::new(Int64Array::from(vec![30, 25, 35, 28, 32]));
  let score_array = Arc::new(Int64Array::from(vec![95, 87, 92, 88, 90]));

  let batch = RecordBatch::try_new(
    schema.clone(),
    vec![
      id_array as ArrayRef,
      name_array as ArrayRef,
      email_array as ArrayRef,
      age_array as ArrayRef,
      score_array as ArrayRef,
    ],
  )?;

  // Write the Parquet file
  let write_consumer = ParquetConsumer::new("examples/file_formats/data/sample.parquet")
    .with_name("parquet-writer".to_string())
    .with_error_strategy(ErrorStrategy::Skip);

  let write_producer = streamweave::producers::vec::vec_producer::VecProducer::new(vec![batch]);
  let write_pipeline = PipelineBuilder::new()
    .with_producer(write_producer)
    .with_consumer(write_consumer)
    .build();

  write_pipeline.run().await?;
  println!("✅ Sample Parquet file created!");

  // Now read with column projection (only id, name, age - skip email and score)
  println!("\n🔄 Reading Parquet with column projection (id, name, age only)...");
  let producer = streamweave::producers::parquet::parquet_producer::ParquetProducer::new(
    "examples/file_formats/data/sample.parquet",
  )
  .with_name("parquet-reader".to_string())
  .with_projection(vec![0, 1, 3]) // Project columns: id (0), name (1), age (3)
  .with_batch_size(100)
  .with_error_strategy(ErrorStrategy::Skip);

  // Transform RecordBatch to display format
  let transformer = MapTransformer::new(|batch: RecordBatch| -> String {
    let mut result = String::new();
    result.push_str(&format!("Batch with {} rows:\n", batch.num_rows()));

    // Get projected columns
    let id_col = batch.column(0);
    let name_col = batch.column(1);
    let age_col = batch.column(2);

    for i in 0..batch.num_rows() {
      let id = id_col
        .as_any()
        .downcast_ref::<Int64Array>()
        .map(|arr| arr.value(i))
        .unwrap_or(0);
      let name = name_col
        .as_any()
        .downcast_ref::<StringArray>()
        .map(|arr| arr.value(i).to_string())
        .unwrap_or_else(|| "N/A".to_string());
      let age = age_col
        .as_any()
        .downcast_ref::<Int64Array>()
        .map(|arr| arr.value(i))
        .unwrap_or(0);

      result.push_str(&format!(
        "  Row {}: ID={}, Name={}, Age={}\n",
        i + 1,
        id,
        name,
        age
      ));
    }

    result
  })
  .with_error_strategy(ErrorStrategy::Skip);

  let consumer = VecConsumer::new();

  let pipeline = PipelineBuilder::new()
    .with_producer(producer)
    .with_transformer(transformer)
    .with_consumer(consumer)
    .build();

  let results = pipeline.run().await?;

  println!("\n✅ Parquet read with projection completed!");
  println!("📊 Results ({} batches):", results.len());
  for result in results {
    print!("{}", result);
  }

  println!("\n💡 Note: Only projected columns (id, name, age) were read from disk!");
  println!("   Email and score columns were skipped, saving I/O and memory.");

  Ok(())
}

# File Formats Integration Example

This example demonstrates how to use StreamWeave with various file formats (CSV, JSONL, and Parquet) for efficient data processing. It shows streaming parsing to avoid loading entire files into memory.

## Prerequisites

Before running this example, you need:

1. **File formats feature enabled** - Build with `--features file-formats`
2. **Test data files** - Sample files are included in `examples/file_formats/data/`

## Quick Start

```bash
# Run CSV example
cargo run --example file_formats --features file-formats csv

# Run JSONL example
cargo run --example file_formats --features file-formats jsonl

# Run Parquet example
cargo run --example file_formats --features file-formats parquet
```

## Running Examples

### 1. CSV Read/Write with Headers

This example demonstrates CSV file processing:

```bash
cargo run --example file_formats --features file-formats csv
```

**What it demonstrates:**
- Reading CSV files with header rows
- Streaming CSV parsing (doesn't load entire file into memory)
- Writing CSV files with headers
- Handling different data types (strings, numbers, booleans)
- Error handling for malformed rows

**Configuration:**
- Reads from `examples/file_formats/data/sample.csv`
- Writes to `examples/file_formats/data/output.csv`
- Uses header row for column mapping
- Skips malformed rows on error

**Key Features:**
- **Streaming Parsing**: Processes CSV row-by-row, not loading entire file
- **Header Handling**: Automatically maps columns using header row
- **Type Safety**: Deserializes to strongly-typed structs
- **Error Resilience**: Configurable error handling (skip, stop, retry)

### 2. JSONL Streaming for Large Files

This example demonstrates JSON Lines (JSONL) processing:

```bash
cargo run --example file_formats --features file-formats jsonl
```

**What it demonstrates:**
- Streaming JSONL parsing (line-by-line)
- Efficient processing of large JSONL files
- Writing JSONL files
- Handling one JSON object per line

**Configuration:**
- Reads from `examples/file_formats/data/sample.jsonl`
- Writes to `examples/file_formats/data/output.jsonl`
- Processes one line at a time (memory efficient)

**Key Features:**
- **Line-by-Line Processing**: Each line is a separate JSON object
- **Memory Efficient**: Only loads one line at a time
- **Large File Support**: Can handle files larger than available memory
- **Streaming**: Processes as data arrives, not waiting for entire file

**Use Cases:**
- Log file processing
- Event stream files
- Large JSON datasets
- Real-time data ingestion

### 3. Parquet Column Projection

This example demonstrates Parquet file processing with column projection:

```bash
cargo run --example file_formats --features file-formats parquet
```

**What it demonstrates:**
- Reading Parquet files with column projection (only selected columns)
- Efficient columnar reading (skips unneeded columns)
- Working with Arrow RecordBatches
- Writing Parquet files
- Batch-based processing

**Configuration:**
- Creates sample Parquet file with 5 columns (id, name, email, age, score)
- Reads with projection: only columns 0, 1, 3 (id, name, age)
- Processes in batches of 100 rows
- Skips email and score columns to save I/O

**Key Features:**
- **Column Projection**: Only reads needed columns from disk
- **Columnar Format**: Efficient for analytics workloads
- **Batch Processing**: Processes data in Arrow RecordBatches
- **I/O Optimization**: Reduces disk I/O by skipping columns

**Benefits:**
- **Faster Reads**: Only reads columns you need
- **Less Memory**: Doesn't load unused columns
- **Better Performance**: Columnar format is optimized for analytics
- **Scalable**: Handles large files efficiently

## Example Output

### CSV Example
```
🚀 StreamWeave File Formats Integration Example
================================================

Running: CSV Read/Write Example
-------------------------------
📄 Setting up CSV read/write example...
🔄 Reading CSV file and processing...
✅ CSV read completed!
📊 Results (5 records):
  1. Person #1: Alice (alice@example.com) - Age: 30, Active: true
  2. Person #2: Bob (bob@example.com) - Age: 25, Active: true
  ...

📝 Writing CSV file...
✅ CSV write completed! Check examples/file_formats/data/output.csv
```

### JSONL Example
```
🚀 StreamWeave File Formats Integration Example
================================================

Running: JSONL Streaming Example
--------------------------------
📄 Setting up JSONL streaming example...
🔄 Streaming JSONL file (line-by-line processing)...
✅ JSONL read completed!
📊 Results (5 records):
  1. Person #1: Alice - Age: 30
  2. Person #2: Bob - Age: 25
  ...

📝 Writing JSONL file...
✅ JSONL write completed! Check examples/file_formats/data/output.jsonl
```

### Parquet Example
```
🚀 StreamWeave File Formats Integration Example
================================================

Running: Parquet Column Projection Example
------------------------------------------
📄 Setting up Parquet column projection example...
📝 Creating sample Parquet file...
✅ Sample Parquet file created!

🔄 Reading Parquet with column projection (id, name, age only)...
✅ Parquet read with projection completed!
📊 Results (1 batches):
Batch with 5 rows:
  Row 1: ID=1, Name=Alice, Age=30
  Row 2: ID=2, Name=Bob, Age=25
  ...

💡 Note: Only projected columns (id, name, age) were read from disk!
   Email and score columns were skipped, saving I/O and memory.
```

## Configuration Options

### CSV Producer

```rust
CsvProducer::<Person>::new("data.csv")
    .with_headers(true)           // File has header row
    .with_delimiter(b',')         // Comma delimiter
    .with_flexible(false)         // Strict column count
    .with_trim(true)              // Trim whitespace
    .with_error_strategy(ErrorStrategy::Skip)
```

### CSV Consumer

```rust
CsvConsumer::<Person>::new("output.csv")
    .with_headers(true)           // Write header row
    .with_delimiter(b',')         // Comma delimiter
    .with_flush_on_write(false)   // Buffer writes
    .with_error_strategy(ErrorStrategy::Skip)
```

### JSONL Producer

```rust
JsonlProducer::<Person>::new("data.jsonl")
    .with_error_strategy(ErrorStrategy::Skip)
```

### JSONL Consumer

```rust
JsonlConsumer::<Person>::new("output.jsonl")
    .with_append(false)           // Overwrite existing file
    .with_buffer_size(8192)       // 8KB buffer
    .with_error_strategy(ErrorStrategy::Skip)
```

### Parquet Producer

```rust
ParquetProducer::new("data.parquet")
    .with_projection(vec![0, 1, 3])  // Read only columns 0, 1, 3
    .with_batch_size(1024)          // 1024 rows per batch
    .with_row_groups(vec![0, 1])    // Read only specific row groups
    .with_error_strategy(ErrorStrategy::Skip)
```

### Parquet Consumer

```rust
ParquetConsumer::new("output.parquet")
    .with_compression(ParquetCompression::Snappy)
    .with_max_row_group_size(1024 * 1024)  // 1MB row groups
    .with_error_strategy(ErrorStrategy::Skip)
```

## Performance Considerations

### CSV
- **Streaming**: Processes row-by-row, memory efficient
- **Headers**: Header row is read once, not per row
- **Parsing**: Efficient CSV parsing with error recovery

### JSONL
- **Line-by-Line**: Each line processed independently
- **Memory**: Constant memory usage regardless of file size
- **Buffering**: Configurable buffer size for I/O optimization

### Parquet
- **Columnar**: Only reads columns you need (projection)
- **Batch Processing**: Processes in configurable batch sizes
- **Compression**: Built-in compression support (Snappy, LZ4, Zstd)
- **Row Groups**: Can read specific row groups for parallel processing

## Use Cases

### CSV
- Data import/export
- ETL pipelines
- Reporting
- Data transformation

### JSONL
- Log processing
- Event streaming
- Large JSON datasets
- Real-time data ingestion

### Parquet
- Analytics workloads
- Data warehousing
- Columnar queries
- Big data processing

## Best Practices

1. **Use Streaming**: Always use streaming parsers for large files
2. **Column Projection**: For Parquet, only read columns you need
3. **Error Handling**: Configure appropriate error strategies
4. **Batch Sizes**: Tune batch sizes for your data and memory constraints
5. **Compression**: Use compression for Parquet to save disk space
6. **Headers**: Always use headers in CSV for clarity and type safety

## Troubleshooting

### CSV Parsing Errors
- Check delimiter matches file format
- Verify header row exists if `with_headers(true)`
- Use `with_flexible(true)` for variable column counts
- Enable `with_trim(true)` to handle whitespace issues

### JSONL Errors
- Ensure each line is valid JSON
- Check for trailing commas or syntax errors
- Use error strategy to skip malformed lines

### Parquet Errors
- Verify column indices in projection are valid
- Check batch size doesn't exceed available memory
- Ensure schema matches when writing

## Next Steps

- Explore other StreamWeave examples
- Combine file formats with transformers for data transformation
- Use with other producers/consumers for complex pipelines
- See the main StreamWeave documentation for advanced features


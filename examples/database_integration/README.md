# Database Integration Example

This example demonstrates how to use StreamWeave with SQL databases for querying and streaming results. It shows how to work with in-memory SQLite databases, parameterized queries, large result sets, and connection pooling.

## Prerequisites

Before running this example, you need:

1. **Database feature enabled** - Build with `--features database`
2. **SQLite support** - SQLite is included with the database feature (no external setup needed for in-memory databases)

## Quick Start

The example uses **in-memory SQLite**, so no external database setup is required! The example automatically creates and populates the database.

```bash
# Run the basic query example
cargo run --example database_integration --features database basic

# Run the parameterized query example
cargo run --example database_integration --features database parameterized

# Run the large result set example
cargo run --example database_integration --features database large

# Run the connection pooling example
cargo run --example database_integration --features database pooling
```

## Examples

### 1. Basic Database Query

Demonstrates:
- Creating an in-memory SQLite database
- Executing a simple SELECT query
- Streaming query results through a pipeline
- Displaying results to the console

```bash
cargo run --example database_integration --features database basic
```

**Output:**
```
🚀 StreamWeave Database Integration Example
===========================================

Running: Basic Database Query
-----------------------------
📊 Setting up basic database query example...
🚀 Starting pipeline to query database...
   Database: SQLite (in-memory)
   Query: SELECT id, name, email, age FROM users

User #1: Alice (alice@example.com) - Age: 30
User #2: Bob (bob@example.com) - Age: 25
User #3: Charlie (charlie@example.com) - Age: 35
User #4: Diana (diana@example.com) - Age: 28
User #5: Eve (eve@example.com) - Age: 32

✅ Basic query example completed!
```

### 2. Parameterized Queries

Demonstrates:
- Using parameterized queries to prevent SQL injection
- Binding parameters to queries
- Filtering results based on parameters

```bash
cargo run --example database_integration --features database parameterized
```

**Key Features:**
- SQLite uses `?` for parameter placeholders
- Parameters are bound safely to prevent SQL injection
- Query filters users older than 25

### 3. Large Result Set Streaming

Demonstrates:
- Cursor-based streaming for large datasets
- Memory-efficient processing of large result sets
- Connection pooling configuration

```bash
cargo run --example database_integration --features database large
```

**Key Features:**
- Processes 1000 rows efficiently
- Memory usage remains bounded due to cursor-based streaming
- Fetch size of 500 rows per batch for optimal performance

### 4. Connection Pooling

Demonstrates:
- Configuring connection pool settings
- Understanding pool behavior
- Optimizing for different workloads

```bash
cargo run --example database_integration --features database pooling
```

**Configuration Options:**
- `max_connections`: Maximum number of connections in the pool
- `fetch_size`: Number of rows fetched per batch (cursor-based streaming)
- Connection pool manages connections efficiently

## Database Producer Configuration

The `DatabaseProducerConfig` provides extensive configuration options:

```rust
let db_config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:?cache=shared".to_string())
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM users WHERE age > ?")
    .with_parameter(serde_json::Value::Number(25.into()))
    .with_max_connections(10)      // Maximum connections in pool
    .with_fetch_size(500)           // Rows per fetch (cursor-based)
    .with_ssl(false);               // Enable SSL/TLS
```

### Connection URL Formats

**SQLite (in-memory):**
```
sqlite::memory:?cache=shared
```

**SQLite (file-based):**
```
sqlite:./database.db
```

**PostgreSQL:**
```
postgresql://user:password@localhost/dbname
```

**MySQL:**
```
mysql://user:password@localhost/dbname
```

## Cursor-Based Streaming

The database producer uses **cursor-based streaming** to efficiently handle large result sets:

- **Fetch Size**: Controls how many rows are fetched per batch
- **Memory Efficiency**: Only a small batch of rows is held in memory at a time
- **Performance**: Larger fetch sizes improve throughput but use more memory

```rust
.with_fetch_size(500)  // Fetch 500 rows per batch
```

## Connection Pooling

Connection pooling improves performance by reusing database connections:

- **Max Connections**: Maximum number of concurrent connections
- **Min Connections**: Minimum number of idle connections to maintain
- **Connection Timeout**: Maximum time to wait for a connection
- **Idle Timeout**: Time before closing idle connections
- **Max Lifetime**: Maximum lifetime of a connection

```rust
.with_max_connections(20)  // Allow up to 20 concurrent connections
```

## Error Handling

The database producer supports the standard StreamWeave error strategies:

```rust
let producer = DatabaseProducer::new(db_config)
    .with_error_strategy(ErrorStrategy::Skip);  // Skip rows that fail
```

**Available Strategies:**
- `ErrorStrategy::Stop` - Stop pipeline on error
- `ErrorStrategy::Skip` - Skip errored rows and continue
- `ErrorStrategy::Retry(n)` - Retry up to n times
- `ErrorStrategy::Custom(handler)` - Custom error handling

## Database Row Access

The `DatabaseRow` type provides easy access to query results:

```rust
let id = row.get("id").and_then(|v| v.as_i64()).unwrap_or(0);
let name = row.get("name").and_then(|v| v.as_str()).unwrap_or("N/A");
let age = row.get("age").and_then(|v| v.as_i64()).unwrap_or(0);
```

**Methods:**
- `get(column)` - Get a value by column name
- `columns()` - Get all column names
- `fields` - Direct access to the HashMap of fields

## Using with Other Databases

While this example uses SQLite, the database producer supports PostgreSQL and MySQL as well:

```rust
// PostgreSQL
let db_config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT * FROM users WHERE age > $1")  // PostgreSQL uses $1, $2, etc.
    .with_parameter(serde_json::Value::Number(25.into()));

// MySQL
let db_config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM users WHERE age > ?")  // MySQL uses ?
    .with_parameter(serde_json::Value::Number(25.into()));
```

## Troubleshooting

### Error: "Database feature is not enabled"

**Solution:** Build with the `database` feature:
```bash
cargo run --example database_integration --features database
```

### Error: "Failed to create database connection pool"

**Solution:** 
- For SQLite: Ensure the connection URL is correct
- For PostgreSQL/MySQL: Verify the database is running and credentials are correct

### Error: "Failed to execute query"

**Solution:**
- Check that the SQL query syntax is correct for your database type
- Verify table and column names exist
- For parameterized queries, ensure parameter count matches placeholders

## Next Steps

- Try modifying the queries to filter or transform data
- Experiment with different fetch sizes to optimize performance
- Add transformers to process and transform database rows
- Connect to a real PostgreSQL or MySQL database
- Use the database producer in a real application pipeline

## Related Examples

- [Basic Pipeline](../basic_pipeline/README.md) - Learn the fundamentals
- [Advanced Pipeline](../advanced_pipeline/README.md) - Complex transformations
- [Kafka Integration](../kafka_integration/README.md) - Message streaming


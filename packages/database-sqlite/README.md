# streamweave-database-sqlite

[![Crates.io](https://img.shields.io/crates/v/streamweave-database-sqlite.svg)](https://crates.io/crates/streamweave-database-sqlite)
[![Documentation](https://docs.rs/streamweave-database-sqlite/badge.svg)](https://docs.rs/streamweave-database-sqlite)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**SQLite integration for StreamWeave**  
*Read from and write to SQLite databases with streaming processing.*

The `streamweave-database-sqlite` package provides SQLite producers and consumers for StreamWeave. It enables querying SQLite databases and inserting data into SQLite tables with connection pooling, batch operations, and transaction support.

## ✨ Key Features

- **SqliteProducer**: Query SQLite databases and stream results
- **SqliteConsumer**: Insert data into SQLite tables
- **Connection Pooling**: Efficient connection management
- **Batch Operations**: Batch inserts for performance
- **Transaction Support**: Transactional batch operations
- **In-Memory Support**: Support for in-memory SQLite databases

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-database-sqlite = "0.3.0"
```

## 🚀 Quick Start

### Query SQLite Database

```rust
use streamweave_database_sqlite::SqliteProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite:data.db")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT id, name, email FROM users");

let pipeline = PipelineBuilder::new()
    .producer(SqliteProducer::new(config)?)
    .consumer(/* process rows */);

pipeline.run().await?;
```

### Insert into SQLite

```rust
use streamweave_database_sqlite::SqliteConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite:data.db")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_batch_size(1000);

let pipeline = PipelineBuilder::new()
    .producer(/* produce rows */)
    .consumer(SqliteConsumer::new(config)?);

pipeline.run().await?;
```

## 📖 API Overview

### SqliteProducer

Queries SQLite databases and streams results:

```rust
pub struct SqliteProducer {
    // Internal state
}
```

**Key Methods:**
- `new(config)` - Create producer with configuration
- `produce()` - Generate stream from query results

### SqliteConsumer

Inserts data into SQLite tables:

```rust
pub struct SqliteConsumer {
    // Internal state
}
```

**Key Methods:**
- `new(config)` - Create consumer with configuration
- `consume(stream)` - Insert stream items into table

## 📚 Usage Examples

### Query with Parameters

Use parameterized queries:

```rust
use streamweave_database_sqlite::SqliteProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};

let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite:data.db")
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM users WHERE age > ? AND city = ?")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));

let producer = SqliteProducer::new(config)?;
```

### In-Memory Database

Use in-memory SQLite database:

```rust
use streamweave_database_sqlite::SqliteProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};

let config = DatabaseProducerConfig::default()
    .with_connection_url("sqlite::memory:")  // In-memory database
    .with_database_type(DatabaseType::Sqlite)
    .with_query("SELECT * FROM users");

let producer = SqliteProducer::new(config)?;
```

### Batch Insertion

Insert data in batches:

```rust
use streamweave_database_sqlite::SqliteConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite:data.db")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(5))
    .with_transactions(true);

let consumer = SqliteConsumer::new(config)?;
```

### Transaction Handling

Use transactions for atomicity:

```rust
use streamweave_database_sqlite::SqliteConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};

let config = DatabaseConsumerConfig::default()
    .with_connection_url("sqlite:data.db")
    .with_database_type(DatabaseType::Sqlite)
    .with_table_name("users")
    .with_transactions(true)
    .with_transaction_size(Some(5000));  // Commit every 5000 rows

let consumer = SqliteConsumer::new(config)?;
```

## 🏗️ Architecture

SQLite integration:

```text
┌──────────┐
│ SQLite  │───> SqliteProducer ───> Stream<DatabaseRow> ───> Transformer ───> Stream<DatabaseRow> ───> SqliteConsumer ───> SQLite
└──────────┘                                                                                                                  └──────────┘
```

**SQLite Flow:**
1. SqliteProducer executes query and streams results
2. DatabaseRow items flow through transformers
3. SqliteConsumer inserts rows into SQLite table
4. Connection pooling manages database connections

## 🔧 Configuration

### Connection URL Format

SQLite connection URL:
```
sqlite:path/to/database.db
sqlite::memory:  # In-memory database
```

### Producer Configuration

- **Connection URL**: SQLite connection string
- **Query**: SQL SELECT query
- **Parameters**: Query parameters (? placeholders)
- **Fetch Size**: Rows per fetch
- **Connection Pool**: Pool configuration

### Consumer Configuration

- **Connection URL**: SQLite connection string
- **Table Name**: Target table for inserts
- **Batch Size**: Rows per batch
- **Transactions**: Use transactions
- **Column Mapping**: Map fields to columns

## 🔍 Error Handling

SQLite errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(SqliteProducer::new(config)?)
    .consumer(SqliteConsumer::new(consumer_config)?);
```

## ⚡ Performance Considerations

- **Connection Pooling**: Reuse connections for efficiency
- **Batch Inserts**: Batch operations for better performance
- **Transactions**: Use transactions for atomicity
- **In-Memory**: In-memory databases are very fast

## 📝 Examples

For more examples, see:
- [Database Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/database_integration)
- [SQLite-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-database-sqlite` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-database` - Base database types
- `streamweave-message` (optional) - Message envelope support
- `sqlx` - SQL database library with SQLite support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support

## 🎯 Use Cases

SQLite integration is used for:

1. **Data Extraction**: Query and extract data from SQLite
2. **Data Loading**: Load data into SQLite tables
3. **ETL Pipelines**: Extract, transform, load from/to SQLite
4. **Embedded Databases**: Use SQLite for embedded applications
5. **Testing**: Use SQLite for testing and development

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-database-sqlite)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/database-sqlite)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-database](../database/README.md) - Base database types
- [streamweave-database-postgresql](../database-postgresql/README.md) - PostgreSQL integration
- [streamweave-database-mysql](../database-mysql/README.md) - MySQL integration

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.


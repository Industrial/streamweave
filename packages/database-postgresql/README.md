# streamweave-database-postgresql

[![Crates.io](https://img.shields.io/crates/v/streamweave-database-postgresql.svg)](https://crates.io/crates/streamweave-database-postgresql)
[![Documentation](https://docs.rs/streamweave-database-postgresql/badge.svg)](https://docs.rs/streamweave-database-postgresql)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**PostgreSQL integration for StreamWeave**  
*Read from and write to PostgreSQL databases with streaming processing.*

The `streamweave-database-postgresql` package provides PostgreSQL producers and consumers for StreamWeave. It enables querying PostgreSQL databases and inserting data into PostgreSQL tables with connection pooling, batch operations, and transaction support.

## ✨ Key Features

- **PostgresProducer**: Query PostgreSQL databases and stream results
- **PostgresConsumer**: Insert data into PostgreSQL tables
- **Connection Pooling**: Efficient connection management
- **Batch Operations**: Batch inserts for performance
- **Transaction Support**: Transactional batch operations
- **Parameterized Queries**: Safe parameterized SQL queries

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-database-postgresql = "0.3.0"
```

## 🚀 Quick Start

### Query PostgreSQL Database

```rust
use streamweave_database_postgresql::PostgresProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT id, name, email FROM users");

let pipeline = PipelineBuilder::new()
    .producer(PostgresProducer::new(config)?)
    .consumer(/* process rows */);

pipeline.run().await?;
```

### Insert into PostgreSQL

```rust
use streamweave_database_postgresql::PostgresConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_batch_size(1000);

let pipeline = PipelineBuilder::new()
    .producer(/* produce rows */)
    .consumer(PostgresConsumer::new(config)?);

pipeline.run().await?;
```

## 📖 API Overview

### PostgresProducer

Queries PostgreSQL databases and streams results:

```rust
pub struct PostgresProducer {
    // Internal state
}
```

**Key Methods:**
- `new(config)` - Create producer with configuration
- `produce()` - Generate stream from query results

### PostgresConsumer

Inserts data into PostgreSQL tables:

```rust
pub struct PostgresConsumer {
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
use streamweave_database_postgresql::PostgresProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};

let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT * FROM users WHERE age > $1 AND city = $2")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));

let producer = PostgresProducer::new(config)?;
```

### Batch Insertion

Insert data in batches:

```rust
use streamweave_database_postgresql::PostgresConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(5))
    .with_transactions(true);

let consumer = PostgresConsumer::new(config)?;
```

### Transaction Handling

Use transactions for atomicity:

```rust
use streamweave_database_postgresql::PostgresConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};

let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_transactions(true)
    .with_transaction_size(Some(5000));  // Commit every 5000 rows

let consumer = PostgresConsumer::new(config)?;
```

### Connection Pooling

Configure connection pool:

```rust
use streamweave_database_postgresql::PostgresProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_max_connections(50)
    .with_min_connections(5)
    .with_connect_timeout(Duration::from_secs(10));

let producer = PostgresProducer::new(config)?;
```

## 🏗️ Architecture

PostgreSQL integration:

```
┌──────────────┐
│ PostgreSQL   │───> PostgresProducer ───> Stream<DatabaseRow> ───> Transformer ───> Stream<DatabaseRow> ───> PostgresConsumer ───> PostgreSQL
└──────────────┘                                                                                                                      └──────────────┘
```

**PostgreSQL Flow:**
1. PostgresProducer executes query and streams results
2. DatabaseRow items flow through transformers
3. PostgresConsumer inserts rows into PostgreSQL table
4. Connection pooling manages database connections

## 🔧 Configuration

### Connection URL Format

PostgreSQL connection URL:
```
postgresql://user:password@host:port/database
```

### Producer Configuration

- **Connection URL**: PostgreSQL connection string
- **Query**: SQL SELECT query
- **Parameters**: Query parameters ($1, $2, etc.)
- **Fetch Size**: Rows per fetch
- **Connection Pool**: Pool configuration

### Consumer Configuration

- **Connection URL**: PostgreSQL connection string
- **Table Name**: Target table for inserts
- **Batch Size**: Rows per batch
- **Transactions**: Use transactions
- **Column Mapping**: Map fields to columns

## 🔍 Error Handling

PostgreSQL errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(PostgresProducer::new(config)?)
    .consumer(PostgresConsumer::new(consumer_config)?);
```

## ⚡ Performance Considerations

- **Connection Pooling**: Reuse connections for efficiency
- **Batch Inserts**: Batch operations for better performance
- **Transactions**: Use transactions for atomicity
- **Fetch Size**: Tune fetch size for query performance

## 📝 Examples

For more examples, see:
- [Database Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/database_integration)
- [PostgreSQL-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-database-postgresql` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-database` - Base database types
- `streamweave-message` (optional) - Message envelope support
- `sqlx` - SQL database library with PostgreSQL support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support

## 🎯 Use Cases

PostgreSQL integration is used for:

1. **Data Extraction**: Query and extract data from PostgreSQL
2. **Data Loading**: Load data into PostgreSQL tables
3. **ETL Pipelines**: Extract, transform, load from/to PostgreSQL
4. **Real-Time Processing**: Process PostgreSQL data in real-time
5. **Data Migration**: Migrate data between systems

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-database-postgresql)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/database-postgresql)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-database](../database/README.md) - Base database types
- [streamweave-database-mysql](../database-mysql/README.md) - MySQL integration
- [streamweave-database-sqlite](../database-sqlite/README.md) - SQLite integration

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.


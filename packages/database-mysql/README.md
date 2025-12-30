# streamweave-database-mysql

[![Crates.io](https://img.shields.io/crates/v/streamweave-database-mysql.svg)](https://crates.io/crates/streamweave-database-mysql)
[![Documentation](https://docs.rs/streamweave-database-mysql/badge.svg)](https://docs.rs/streamweave-database-mysql)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**MySQL integration for StreamWeave**  
*Read from and write to MySQL databases with streaming processing.*

The `streamweave-database-mysql` package provides MySQL producers and consumers for StreamWeave. It enables querying MySQL databases and inserting data into MySQL tables with connection pooling, batch operations, and transaction support.

## ✨ Key Features

- **MysqlProducer**: Query MySQL databases and stream results
- **MysqlConsumer**: Insert data into MySQL tables
- **Connection Pooling**: Efficient connection management
- **Batch Operations**: Batch inserts for performance
- **Transaction Support**: Transactional batch operations
- **Parameterized Queries**: Safe parameterized SQL queries

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-database-mysql = "0.6.0"
```

## 🚀 Quick Start

### Query MySQL Database

```rust
use streamweave_database_mysql::MysqlProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT id, name, email FROM users");

let pipeline = PipelineBuilder::new()
    .producer(MysqlProducer::new(config)?)
    .consumer(/* process rows */);

pipeline.run().await?;
```

### Insert into MySQL

```rust
use streamweave_database_mysql::MysqlConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use streamweave_pipeline::PipelineBuilder;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_batch_size(1000);

let pipeline = PipelineBuilder::new()
    .producer(/* produce rows */)
    .consumer(MysqlConsumer::new(config)?);

pipeline.run().await?;
```

## 📖 API Overview

### MysqlProducer

Queries MySQL databases and streams results:

```rust
pub struct MysqlProducer {
    // Internal state
}
```

**Key Methods:**
- `new(config)` - Create producer with configuration
- `produce()` - Generate stream from query results

### MysqlConsumer

Inserts data into MySQL tables:

```rust
pub struct MysqlConsumer {
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
use streamweave_database_mysql::MysqlProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};

let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_query("SELECT * FROM users WHERE age > ? AND city = ?")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));

let producer = MysqlProducer::new(config)?;
```

### Batch Insertion

Insert data in batches:

```rust
use streamweave_database_mysql::MysqlConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(5))
    .with_transactions(true);

let consumer = MysqlConsumer::new(config)?;
```

### Transaction Handling

Use transactions for atomicity:

```rust
use streamweave_database_mysql::MysqlConsumer;
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};

let config = DatabaseConsumerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_table_name("users")
    .with_transactions(true)
    .with_transaction_size(Some(5000));  // Commit every 5000 rows

let consumer = MysqlConsumer::new(config)?;
```

### Connection Pooling

Configure connection pool:

```rust
use streamweave_database_mysql::MysqlProducer;
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseProducerConfig::default()
    .with_connection_url("mysql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Mysql)
    .with_max_connections(50)
    .with_min_connections(5)
    .with_connect_timeout(Duration::from_secs(10));

let producer = MysqlProducer::new(config)?;
```

## 🏗️ Architecture

MySQL integration:

```text
┌──────────┐
│  MySQL   │───> MysqlProducer ───> Stream<DatabaseRow> ───> Transformer ───> Stream<DatabaseRow> ───> MysqlConsumer ───> MySQL
└──────────┘                                                                                                                  └──────────┘
```

**MySQL Flow:**
1. MysqlProducer executes query and streams results
2. DatabaseRow items flow through transformers
3. MysqlConsumer inserts rows into MySQL table
4. Connection pooling manages database connections

## 🔧 Configuration

### Connection URL Format

MySQL connection URL:
```
mysql://user:password@host:port/database
```

### Producer Configuration

- **Connection URL**: MySQL connection string
- **Query**: SQL SELECT query
- **Parameters**: Query parameters
- **Fetch Size**: Rows per fetch
- **Connection Pool**: Pool configuration

### Consumer Configuration

- **Connection URL**: MySQL connection string
- **Table Name**: Target table for inserts
- **Batch Size**: Rows per batch
- **Transactions**: Use transactions
- **Column Mapping**: Map fields to columns

## 🔍 Error Handling

MySQL errors are handled through the error system:

```rust
use streamweave_error::ErrorStrategy;

let pipeline = PipelineBuilder::new()
    .with_error_strategy(ErrorStrategy::Skip)
    .producer(MysqlProducer::new(config)?)
    .consumer(MysqlConsumer::new(consumer_config)?);
```

## ⚡ Performance Considerations

- **Connection Pooling**: Reuse connections for efficiency
- **Batch Inserts**: Batch operations for better performance
- **Transactions**: Use transactions for atomicity
- **Fetch Size**: Tune fetch size for query performance

## 📝 Examples

For more examples, see:
- [Database Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/database_integration)
- [MySQL-Specific Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-database-mysql` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `streamweave-database` - Base database types
- `streamweave-message` (optional) - Message envelope support
- `sqlx` - SQL database library with MySQL support
- `tokio` - Async runtime
- `futures` - Stream utilities
- `serde` - Serialization support

## 🎯 Use Cases

MySQL integration is used for:

1. **Data Extraction**: Query and extract data from MySQL
2. **Data Loading**: Load data into MySQL tables
3. **ETL Pipelines**: Extract, transform, load from/to MySQL
4. **Real-Time Processing**: Process MySQL data in real-time
5. **Data Migration**: Migrate data between systems

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-database-mysql)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/database-mysql)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-database](../database/README.md) - Base database types
- [streamweave-database-postgresql](../database-postgresql/README.md) - PostgreSQL integration
- [streamweave-database-sqlite](../database-sqlite/README.md) - SQLite integration

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.


# streamweave-database

[![Crates.io](https://img.shields.io/crates/v/streamweave-database.svg)](https://crates.io/crates/streamweave-database)
[![Documentation](https://docs.rs/streamweave-database/badge.svg)](https://docs.rs/streamweave-database)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**Base types for StreamWeave database packages**  
*Common types and configuration for database integration.*

The `streamweave-database` package provides common types and configuration structures used by database-specific implementations. It defines the base abstractions for database producers and consumers that are implemented by specific database packages.

## ✨ Key Features

- **DatabaseType Enum**: Support for PostgreSQL, MySQL, SQLite
- **DatabaseRow Type**: Generic row representation
- **DatabaseProducerConfig**: Configuration for database query producers
- **DatabaseConsumerConfig**: Configuration for database insert consumers
- **Connection Pooling**: Connection pool configuration
- **Query Configuration**: Query and parameter configuration

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-database = "0.6.0"
```

## 🚀 Quick Start

### Using Database Types

```rust
use streamweave_database::{DatabaseType, DatabaseRow, DatabaseProducerConfig, DatabaseConsumerConfig};

// Configure producer
let producer_config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT * FROM users");

// Configure consumer
let consumer_config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/dbname")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users");
```

## 📖 API Overview

### DatabaseType

Enumeration of supported database types:

```rust
pub enum DatabaseType {
    Postgres,  // PostgreSQL database
    Mysql,     // MySQL/MariaDB database
    Sqlite,    // SQLite database
}
```

### DatabaseRow

Generic representation of a database row:

```rust
pub struct DatabaseRow {
    pub fields: HashMap<String, serde_json::Value>,
}
```

**Key Methods:**
- `new(fields)` - Create new row from fields
- `get(column)` - Get value by column name
- `columns()` - Get all column names

### DatabaseProducerConfig

Configuration for database query producers:

```rust
pub struct DatabaseProducerConfig {
    pub connection_url: String,
    pub database_type: DatabaseType,
    pub query: String,
    pub parameters: Vec<serde_json::Value>,
    pub max_connections: u32,
    pub fetch_size: usize,
    // ... other options
}
```

### DatabaseConsumerConfig

Configuration for database insert consumers:

```rust
pub struct DatabaseConsumerConfig {
    pub connection_url: String,
    pub database_type: DatabaseType,
    pub table_name: String,
    pub batch_size: usize,
    pub use_transactions: bool,
    // ... other options
}
```

## 📚 Usage Examples

### Producer Configuration

Configure a database producer:

```rust
use streamweave_database::{DatabaseProducerConfig, DatabaseType};
use std::time::Duration;

let config = DatabaseProducerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_query("SELECT id, name, email FROM users WHERE active = $1")
    .with_parameter(serde_json::Value::Bool(true))
    .with_max_connections(20)
    .with_fetch_size(500)
    .with_ssl(true);
```

### Consumer Configuration

Configure a database consumer:

```rust
use streamweave_database::{DatabaseConsumerConfig, DatabaseType};
use std::time::Duration;
use std::collections::HashMap;

let mut column_mapping = HashMap::new();
column_mapping.insert("user_id".to_string(), "id".to_string());
column_mapping.insert("user_name".to_string(), "name".to_string());

let config = DatabaseConsumerConfig::default()
    .with_connection_url("postgresql://user:pass@localhost/mydb")
    .with_database_type(DatabaseType::Postgres)
    .with_table_name("users")
    .with_batch_size(1000)
    .with_batch_timeout(Duration::from_secs(10))
    .with_column_mapping(column_mapping)
    .with_transactions(true)
    .with_transaction_size(Some(500));
```

### Working with DatabaseRow

Access row data:

```rust
use streamweave_database::DatabaseRow;
use std::collections::HashMap;

let mut fields = HashMap::new();
fields.insert("id".to_string(), serde_json::Value::Number(1.into()));
fields.insert("name".to_string(), serde_json::Value::String("Alice".to_string()));

let row = DatabaseRow::new(fields);

// Get value by column name
let id = row.get("id");
let name = row.get("name");

// Get all column names
let columns = row.columns();
```

### Parameterized Queries

Use parameterized queries:

```rust
use streamweave_database::DatabaseProducerConfig;

let config = DatabaseProducerConfig::default()
    .with_query("SELECT * FROM users WHERE age > $1 AND city = $2")
    .with_parameter(serde_json::Value::Number(18.into()))
    .with_parameter(serde_json::Value::String("New York".to_string()));
```

### Connection Pooling

Configure connection pooling:

```rust
use streamweave_database::DatabaseProducerConfig;
use std::time::Duration;

let config = DatabaseProducerConfig::default()
    .with_max_connections(50)
    .with_min_connections(5)
    .with_connect_timeout(Duration::from_secs(10))
    .with_idle_timeout(Some(Duration::from_secs(300)))
    .with_max_lifetime(Some(Duration::from_secs(3600)));
```

### Batch Insertion

Configure batch insertion:

```rust
use streamweave_database::DatabaseConsumerConfig;
use std::time::Duration;

let config = DatabaseConsumerConfig::default()
    .with_batch_size(1000)  // Insert 1000 rows at a time
    .with_batch_timeout(Duration::from_secs(5))  // Flush after 5 seconds
    .with_transactions(true)
    .with_transaction_size(Some(5000));  // Commit every 5000 rows
```

## 🏗️ Architecture

Database integration uses base types:

```text
┌──────────────────┐
│ Database Types    │───> Used by database-specific packages
│ - DatabaseType    │
│ - DatabaseRow     │
│ - Configs         │
└──────────────────┘
         │
         ▼
┌──────────────────┐
│ Database-Specific│───> PostgreSQL, MySQL, SQLite
│ Implementations  │
└──────────────────┘
```

**Database Flow:**
1. Base types define common interface
2. Database-specific packages implement producers/consumers
3. Configuration is shared across implementations
4. DatabaseRow provides generic row representation

## 🔧 Configuration

### Producer Configuration Options

- **Connection URL**: Database connection string
- **Database Type**: PostgreSQL, MySQL, or SQLite
- **Query**: SQL query to execute
- **Parameters**: Query parameters
- **Connection Pool**: Pool size and timeouts
- **Fetch Size**: Rows per fetch for cursor iteration
- **SSL**: Enable SSL/TLS

### Consumer Configuration Options

- **Connection URL**: Database connection string
- **Database Type**: PostgreSQL, MySQL, or SQLite
- **Table Name**: Target table for inserts
- **Batch Size**: Rows per batch insert
- **Batch Timeout**: Timeout before flushing batch
- **Column Mapping**: Map field names to column names
- **Transactions**: Use transactions for batch inserts
- **Transaction Size**: Commit every N rows

## 🔍 Error Handling

Database operations use the standard error handling system:

```rust
use streamweave_error::ErrorStrategy;

// Errors are handled through the error system
// Database-specific packages handle connection and query errors
```

## ⚡ Performance Considerations

- **Connection Pooling**: Reuse connections for efficiency
- **Batch Operations**: Batch inserts for better performance
- **Fetch Size**: Tune fetch size for cursor iteration
- **Transactions**: Use transactions for atomicity

## 📝 Examples

For more examples, see:
- [Database Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/database_integration)
- [Database-Specific Packages](../database-postgresql/README.md)

## 🔗 Dependencies

`streamweave-database` depends on:

- `serde` - Serialization support
- `serde_json` - JSON serialization
- `streamweave-message` (optional) - Message envelope support

## 🎯 Use Cases

Database base types are used for:

1. **Type Safety**: Common types across database implementations
2. **Configuration**: Shared configuration structures
3. **Abstraction**: Abstract over different database types
4. **Interoperability**: Work with multiple database backends
5. **Code Reuse**: Share common database logic

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-database)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/database)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-database-postgresql](../database-postgresql/README.md) - PostgreSQL integration
- [streamweave-database-mysql](../database-mysql/README.md) - MySQL integration
- [streamweave-database-sqlite](../database-sqlite/README.md) - SQLite integration

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.


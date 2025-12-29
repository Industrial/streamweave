# streamweave-integration-sql

[![Crates.io](https://img.shields.io/crates/v/streamweave-integration-sql.svg)](https://crates.io/crates/streamweave-integration-sql)
[![Documentation](https://docs.rs/streamweave-integration-sql/badge.svg)](https://docs.rs/streamweave-integration-sql)
[![License: CC BY-SA 4.0](https://img.shields.io/badge/License-CC%20BY--SA%204.0-lightgrey.svg)](https://creativecommons.org/licenses/by-sa/4.0/)

**SQL query support for StreamWeave**  
*Parse and translate SQL queries into StreamWeave pipelines.*

The `streamweave-integration-sql` package provides SQL query support for StreamWeave. It enables parsing SQL queries and translating them into StreamWeave pipeline operations, supporting streaming SQL semantics.

## ✨ Key Features

- **SQL Parser**: Parse SQL queries into AST
- **Query Translator**: Translate SQL to pipeline operations
- **Stream SQL Dialect**: Streaming-specific SQL dialect
- **Query Optimization**: SQL query optimization
- **Multiple Dialects**: Support for different SQL dialects

## 📦 Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
streamweave-integration-sql = { version = "0.3.0", features = ["sql"] }
```

## 🚀 Quick Start

### Parse SQL Query

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, COUNT(*) FROM events GROUP BY user_id"
)?;
```

### Translate to Pipeline

```rust
use streamweave_integration_sql::{SqlParser, QueryTranslator};

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, COUNT(*) FROM events WHERE event_type = 'click' GROUP BY user_id"
)?;

let translator = QueryTranslator::new();
let plan = translator.translate(&query)?;
```

## 📖 API Overview

### SqlParser

Parses SQL queries into AST:

```rust
pub struct SqlParser {
    dialect: GenericDialect,
}
```

**Key Methods:**
- `new()` - Create parser with default dialect
- `parse(query)` - Parse SQL query string

### QueryTranslator

Translates SQL AST to pipeline operations:

```rust
pub struct QueryTranslator {
    // Internal state
}
```

**Key Methods:**
- `new()` - Create translator
- `translate(query)` - Translate SQL query to query plan

### SqlQuery

Represents a parsed SQL query:

```rust
pub struct SqlQuery {
    pub select: SelectClause,
    pub from: FromClause,
    pub where_clause: Option<WhereClause>,
    pub group_by: Option<GroupByClause>,
    pub window: Option<WindowClause>,
    pub order_by: Option<OrderByClause>,
    pub limit: Option<LimitClause>,
}
```

## 📚 Usage Examples

### Basic SELECT Query

Parse and translate a basic SELECT query:

```rust
use streamweave_integration_sql::{SqlParser, QueryTranslator};

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, event_type FROM events"
)?;

let translator = QueryTranslator::new();
let plan = translator.translate(&query)?;
```

### WHERE Clause

Filter with WHERE clause:

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT * FROM events WHERE event_type = 'click' AND timestamp > NOW() - INTERVAL '1 hour'"
)?;
```

### GROUP BY with Aggregation

Group and aggregate:

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, COUNT(*) as event_count, AVG(value) as avg_value
     FROM events
     GROUP BY user_id
     WINDOW TUMBLING (SIZE 1 MINUTE)"
)?;
```

### Window Functions

Use window functions:

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, COUNT(*) 
     FROM events
     GROUP BY user_id
     WINDOW TUMBLING (SIZE 1 MINUTE)"
)?;
```

### ORDER BY and LIMIT

Sort and limit results:

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
let query = parser.parse(
    "SELECT user_id, COUNT(*) as count
     FROM events
     GROUP BY user_id
     ORDER BY count DESC
     LIMIT 10"
)?;
```

## 🏗️ Architecture

SQL integration flow:

```
SQL Query ──> SqlParser ──> SqlQuery AST ──> QueryTranslator ──> QueryPlan ──> Pipeline
```

**SQL Flow:**
1. SQL query string is parsed into AST
2. AST is translated into query plan
3. Query plan describes pipeline operations
4. Pipeline is built from query plan

## 🔧 Configuration

### SQL Dialect

The package supports a Stream SQL dialect that extends standard SQL with streaming-specific constructs:

- **SELECT**: Project columns and compute expressions
- **WHERE**: Filter stream elements
- **GROUP BY**: Aggregate data by grouping keys
- **WINDOW**: Define time-based or count-based windows
- **ORDER BY**: Sort results
- **LIMIT**: Limit number of results

### Supported Aggregations

- `COUNT(*)` - Count of elements
- `COUNT(column)` - Count of non-null values
- `SUM(column)` - Sum of values
- `AVG(column)` - Average of values
- `MIN(column)` - Minimum value
- `MAX(column)` - Maximum value
- `FIRST(column)` - First value in window
- `LAST(column)` - Last value in window

### Window Types

- **Tumbling Windows**: Non-overlapping windows
- **Sliding Windows**: Overlapping windows
- **Session Windows**: Session-based windows

## 🔍 Error Handling

SQL parsing errors are handled:

```rust
use streamweave_integration_sql::SqlParser;

let parser = SqlParser::new();
match parser.parse("SELECT * FROM events") {
    Ok(query) => { /* use query */ }
    Err(e) => { /* handle parsing error */ }
}
```

## ⚡ Performance Considerations

- **Query Optimization**: Queries are optimized before translation
- **Streaming Semantics**: SQL operations are applied continuously
- **Window Processing**: Windows are processed efficiently

## 📝 Examples

For more examples, see:
- [SQL Integration Example](https://github.com/Industrial/streamweave/tree/main/examples/sql_integration)
- [SQL Query Examples](https://github.com/Industrial/streamweave/tree/main/examples)

## 🔗 Dependencies

`streamweave-integration-sql` depends on:

- `streamweave` - Core traits
- `streamweave-error` - Error handling
- `sqlparser` - SQL parsing
- `serde` - Serialization support
- `tokio` - Async runtime
- `futures` - Stream utilities

## 🎯 Use Cases

SQL integration is used for:

1. **SQL Queries**: Write SQL queries for stream processing
2. **Query Translation**: Translate SQL to pipelines
3. **Streaming SQL**: Use SQL for streaming data
4. **Query Optimization**: Optimize SQL queries

## 📖 Documentation

- [Full API Documentation](https://docs.rs/streamweave-integration-sql)
- [Repository](https://github.com/Industrial/streamweave/tree/main/packages/integrations/sql)
- [StreamWeave Main Documentation](https://docs.rs/streamweave)

## 🔗 See Also

- [streamweave](../streamweave/README.md) - Core traits
- [streamweave-error](../../error/README.md) - Error handling
- [streamweave-window](../../window/README.md) - Windowing operations

## 🤝 Contributing

Contributions are welcome! Please see the [Contributing Guide](https://github.com/Industrial/streamweave/blob/main/CONTRIBUTING.md) for details.

## 📄 License

This project is licensed under the [CC BY-SA 4.0](https://creativecommons.org/licenses/by-sa/4.0/) license.


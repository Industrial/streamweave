//! # Stream SQL Dialect Definition
//!
//! This module defines the SQL dialect supported by StreamWeave for stream processing.
//! The dialect extends standard SQL with streaming-specific constructs while maintaining
//! familiar SQL syntax.
//!
//! ## Core SQL Constructs
//!
//! ### SELECT
//!
//! Project columns and compute expressions from stream elements.
//!
//! ```sql
//! SELECT column1, column2, column1 + column2 AS sum
//! FROM stream_name
//! ```
//!
//! **Streaming Semantics**: SELECT operates on each element as it arrives,
//! producing transformed elements immediately.
//!
//! ### WHERE
//!
//! Filter stream elements based on conditions.
//!
//! ```sql
//! SELECT *
//! FROM events
//! WHERE event_type = 'click' AND timestamp > NOW() - INTERVAL '1 hour'
//! ```
//!
//! **Streaming Semantics**: WHERE filters elements as they arrive, similar
//! to traditional SQL but applied continuously.
//!
//! ### GROUP BY
//!
//! Aggregate data by grouping keys. Requires a WINDOW clause for streaming.
//!
//! ```sql
//! SELECT user_id, COUNT(*) as event_count, AVG(value) as avg_value
//! FROM events
//! GROUP BY user_id
//! WINDOW TUMBLING (SIZE 1 MINUTE)
//! ```
//!
//! **Streaming Semantics**: GROUP BY produces results when windows close.
//! Aggregations are computed per window per key.
//!
//! **Supported Aggregations**:
//! - `COUNT(*)` - Count of elements
//! - `COUNT(column)` - Count of non-null values
//! - `SUM(column)` - Sum of values
//! - `AVG(column)` - Average of values
//! - `MIN(column)` - Minimum value
//! - `MAX(column)` - Maximum value
//! - `FIRST(column)` - First value in window
//! - `LAST(column)` - Last value in window
//!
//! ### WINDOW
//!
//! Define time-based or count-based windows for aggregations.
//!
//! **Tumbling Windows** (non-overlapping):
//! ```sql
//! WINDOW TUMBLING (SIZE 1 MINUTE)
//! WINDOW TUMBLING (SIZE 100 ROWS)
//! ```
//!
//! **Sliding Windows** (overlapping):
//! ```sql
//! WINDOW SLIDING (SIZE 1 MINUTE, SLIDE 30 SECONDS)
//! WINDOW SLIDING (SIZE 100 ROWS, SLIDE 50 ROWS)
//! ```
//!
//! **Session Windows** (gap-based):
//! ```sql
//! WINDOW SESSION (GAP 5 MINUTES)
//! ```
//!
//! **Streaming Semantics**: Windows group elements for aggregation. Results
//! are emitted when windows close (for tumbling) or slide (for sliding).
//!
//! ### JOIN
//!
//! Join two streams based on keys. Requires windowing to bound state.
//!
//! ```sql
//! SELECT a.id, a.value, b.value2
//! FROM stream_a a
//! JOIN stream_b b ON a.id = b.id
//! WINDOW TUMBLING (SIZE 1 MINUTE)
//! ```
//!
//! **Streaming Semantics**: JOIN matches elements from both streams within
//! the same window. Elements outside the window are not matched.
//!
//! **Join Types**:
//! - `INNER JOIN` - Only matching elements (default)
//! - `LEFT JOIN` - All elements from left stream
//! - `RIGHT JOIN` - All elements from right stream
//! - `FULL OUTER JOIN` - All elements from both streams
//!
//! ### ORDER BY
//!
//! Sort results. Limited for unbounded streams.
//!
//! ```sql
//! SELECT *
//! FROM events
//! ORDER BY timestamp DESC
//! LIMIT 100
//! ```
//!
//! **Streaming Semantics**: ORDER BY with LIMIT can sort a bounded subset.
//! Without LIMIT, ORDER BY may buffer elements, which can be memory-intensive
//! for unbounded streams.
//!
//! ### LIMIT
//!
//! Limit the number of results.
//!
//! ```sql
//! SELECT *
//! FROM events
//! LIMIT 1000
//! ```
//!
//! **Streaming Semantics**: LIMIT stops the stream after N elements.
//!
//! ## Data Types
//!
//! The dialect supports standard SQL data types:
//!
//! - **Numeric**: `INTEGER`, `BIGINT`, `DOUBLE`, `DECIMAL`
//! - **Text**: `VARCHAR`, `TEXT`
//! - **Boolean**: `BOOLEAN`
//! - **Temporal**: `TIMESTAMP`, `DATE`, `TIME`
//! - **JSON**: `JSON`, `JSONB`
//!
//! ## Functions
//!
//! ### String Functions
//! - `LENGTH(str)` - String length
//! - `UPPER(str)`, `LOWER(str)` - Case conversion
//! - `SUBSTRING(str, start, length)` - Extract substring
//! - `CONCAT(str1, str2, ...)` - Concatenate strings
//!
//! ### Numeric Functions
//! - `ABS(num)` - Absolute value
//! - `ROUND(num, decimals)` - Round to decimals
//! - `FLOOR(num)`, `CEIL(num)` - Floor/ceiling
//!
//! ### Temporal Functions
//! - `NOW()` - Current timestamp
//! - `EXTRACT(field FROM timestamp)` - Extract time component
//! - `DATE_ADD(timestamp, interval)` - Add interval
//! - `DATE_SUB(timestamp, interval)` - Subtract interval
//!
//! ### Window Functions
//! - `ROW_NUMBER()` - Row number within window
//! - `RANK()` - Rank with gaps
//! - `DENSE_RANK()` - Rank without gaps
//!
//! ## Examples
//!
//! ### Simple Filtering
//! ```sql
//! SELECT user_id, event_type, value
//! FROM events
//! WHERE value > 100 AND event_type = 'purchase'
//! ```
//!
//! ### Time-Based Aggregation
//! ```sql
//! SELECT user_id, COUNT(*) as purchase_count, SUM(amount) as total_spent
//! FROM purchases
//! WHERE timestamp > NOW() - INTERVAL '1 day'
//! GROUP BY user_id
//! WINDOW TUMBLING (SIZE 1 HOUR)
//! ```
//!
//! ### Stream Join
//! ```sql
//! SELECT u.user_id, u.name, p.product_name, p.amount
//! FROM users u
//! JOIN purchases p ON u.user_id = p.user_id
//! WINDOW TUMBLING (SIZE 5 MINUTES)
//! ```
//!
//! ### Complex Aggregation
//! ```sql
//! SELECT
//!     category,
//!     COUNT(*) as item_count,
//!     AVG(price) as avg_price,
//!     MIN(price) as min_price,
//!     MAX(price) as max_price
//! FROM products
//! GROUP BY category
//! WINDOW SLIDING (SIZE 10 MINUTES, SLIDE 1 MINUTE)
//! HAVING COUNT(*) > 10
//! ```

/// Stream SQL dialect definition
///
/// This struct defines the SQL dialect supported by StreamWeave,
/// including syntax rules, supported constructs, and streaming semantics.
#[derive(Debug, Clone)]
pub struct StreamSqlDialect {
  /// Whether ORDER BY without LIMIT is allowed (may buffer unbounded data)
  pub allow_unbounded_order_by: bool,
  /// Maximum window size for safety
  pub max_window_size_seconds: Option<u64>,
  /// Maximum window size in rows
  pub max_window_size_rows: Option<u64>,
}

impl Default for StreamSqlDialect {
  fn default() -> Self {
    Self {
      allow_unbounded_order_by: false,
      max_window_size_seconds: Some(86400), // 24 hours
      max_window_size_rows: Some(1_000_000),
    }
  }
}

impl StreamSqlDialect {
  /// Create a new Stream SQL dialect with default settings
  pub fn new() -> Self {
    Self::default()
  }

  /// Create a dialect that allows unbounded ORDER BY (use with caution)
  pub fn with_unbounded_order_by(mut self) -> Self {
    self.allow_unbounded_order_by = true;
    self
  }

  /// Set maximum window size in seconds
  pub fn with_max_window_size_seconds(mut self, seconds: u64) -> Self {
    self.max_window_size_seconds = Some(seconds);
    self
  }

  /// Set maximum window size in rows
  pub fn with_max_window_size_rows(mut self, rows: u64) -> Self {
    self.max_window_size_rows = Some(rows);
    self
  }

  /// Validate window size against limits
  #[allow(clippy::collapsible_if)]
  pub fn validate_window_size(
    &self,
    seconds: Option<u64>,
    rows: Option<u64>,
  ) -> Result<(), String> {
    if let (Some(max_seconds), Some(secs)) = (self.max_window_size_seconds, seconds) {
      if secs > max_seconds {
        return Err(format!(
          "Window size {} seconds exceeds maximum of {} seconds",
          secs, max_seconds
        ));
      }
    }

    if let (Some(max_rows), Some(row_count)) = (self.max_window_size_rows, rows) {
      if row_count > max_rows {
        return Err(format!(
          "Window size {} rows exceeds maximum of {} rows",
          row_count, max_rows
        ));
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_default_dialect() {
    let dialect = StreamSqlDialect::default();
    assert!(!dialect.allow_unbounded_order_by);
    assert_eq!(dialect.max_window_size_seconds, Some(86400));
    assert_eq!(dialect.max_window_size_rows, Some(1_000_000));
  }

  #[test]
  fn test_window_size_validation() {
    let dialect = StreamSqlDialect::default();

    // Valid window sizes
    assert!(dialect.validate_window_size(Some(3600), None).is_ok());
    assert!(dialect.validate_window_size(None, Some(1000)).is_ok());

    // Invalid window sizes
    assert!(dialect.validate_window_size(Some(100000), None).is_err());
    assert!(dialect.validate_window_size(None, Some(2_000_000)).is_err());
  }

  #[test]
  fn test_custom_dialect() {
    let dialect = StreamSqlDialect::new()
      .with_unbounded_order_by()
      .with_max_window_size_seconds(3600)
      .with_max_window_size_rows(10000);

    assert!(dialect.allow_unbounded_order_by);
    assert_eq!(dialect.max_window_size_seconds, Some(3600));
    assert_eq!(dialect.max_window_size_rows, Some(10000));
  }
}
